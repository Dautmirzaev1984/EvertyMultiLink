package ru.everty.multilink.ui

import android.app.Activity
import android.app.UiModeManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.res.Configuration
import android.net.VpnService
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.LayoutInflater
import android.widget.ImageView
import android.widget.TextView
import android.widget.Toast
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.viewpager2.widget.ViewPager2
import com.google.android.material.button.MaterialButton
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import ru.everty.multilink.R
import ru.everty.multilink.databinding.ActivityMainBinding
import ru.everty.multilink.model.AppSettings
import ru.everty.multilink.model.VpnNode
import ru.everty.multilink.parser.ConfigParser
import ru.everty.multilink.qr.QrUtils
import ru.everty.multilink.service.EvertyVpnService
import ru.everty.multilink.sync.TvLanClient
import ru.everty.multilink.sync.TvLanSyncServer

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var settings: AppSettings
    private lateinit var pagerAdapter: MainPagerAdapter
    private val mainHandler = Handler(Looper.getMainLooper())
    private var isTvDevice = false
    private var lanServer: TvLanSyncServer? = null

    private val vpnPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            startVpnService(EvertyVpnService.ACTION_CONNECT)
        } else {
            Toast.makeText(this, "Разрешение на VPN отклонено", Toast.LENGTH_SHORT).show()
        }
    }

    private val qrScannerLauncher = registerForActivityResult(ScanContract()) { result ->
        if (result.contents != null) {
            handleScannedQr(result.contents)
        }
    }

    private val vpnStateReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            val state = intent?.getIntExtra(EvertyVpnService.EXTRA_STATE, EvertyVpnService.STATE_DISCONNECTED)
                ?: EvertyVpnService.STATE_DISCONNECTED
            pagerAdapter.homeFragment.updateVpnState(state)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        settings = AppSettings(this)
        detectDeviceMode()
        setupViewPagerAndNavigation()
        handleIntent(intent)

        // Automatic subscription check & sync on launch
        checkSubscriptionOnStart()
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        handleIntent(intent)
    }

    private fun detectDeviceMode() {
        val uiModeManager = getSystemService(Context.UI_MODE_SERVICE) as? UiModeManager
        val isTelevision = uiModeManager?.currentModeType == Configuration.UI_MODE_TYPE_TELEVISION
        val isTvFeature = packageManager.hasSystemFeature("android.software.leanback")
        isTvDevice = isTelevision || isTvFeature
    }

    private fun setupViewPagerAndNavigation() {
        pagerAdapter = MainPagerAdapter(this)
        binding.viewPager.adapter = pagerAdapter
        binding.viewPager.offscreenPageLimit = 2

        // Bottom Navigation (Phone, Tablet, TV D-Pad)
        binding.bottomNav.setOnItemSelectedListener { item ->
            when (item.itemId) {
                R.id.nav_home -> binding.viewPager.setCurrentItem(0, true)
                R.id.nav_servers -> binding.viewPager.setCurrentItem(1, true)
                R.id.nav_routing -> binding.viewPager.setCurrentItem(2, true)
            }
            true
        }

        // Sync ViewPager swipe with BottomNav selection
        binding.viewPager.registerOnPageChangeCallback(object : ViewPager2.OnPageChangeCallback() {
            override fun onPageSelected(position: Int) {
                when (position) {
                    0 -> binding.bottomNav.selectedItemId = R.id.nav_home
                    1 -> binding.bottomNav.selectedItemId = R.id.nav_servers
                    2 -> binding.bottomNav.selectedItemId = R.id.nav_routing
                }
            }
        })
    }

    private fun checkSubscriptionOnStart() {
        if (settings.isAutoUpdateOnStartEnabled) {
            val subUrl = settings.lastSubscriptionUrl
            if (!subUrl.isNullOrEmpty()) {
                fetchSubscription(subUrl, isManual = false)
            }
        }
    }

    fun toggleVpn() {
        if (EvertyVpnService.currentState == EvertyVpnService.STATE_CONNECTED) {
            startVpnService(EvertyVpnService.ACTION_DISCONNECT)
        } else {
            val prepareIntent = VpnService.prepare(this)
            if (prepareIntent != null) {
                vpnPermissionLauncher.launch(prepareIntent)
            } else {
                startVpnService(EvertyVpnService.ACTION_CONNECT)
            }
        }
    }

    private fun startVpnService(action: String) {
        val intent = Intent(this, EvertyVpnService::class.java).apply {
            this.action = action
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(intent)
        } else {
            startService(intent)
        }
    }

    fun startQrScan() {
        val options = ScanOptions().apply {
            setPrompt("Наведите камеру на QR-код подписки или ТВ")
            setBeepEnabled(true)
            setOrientationLocked(false)
        }
        qrScannerLauncher.launch(options)
    }

    fun showLanSyncDialog() {
        val dialogView = LayoutInflater.from(this).inflate(R.layout.dialog_lan_sync, null)
        val tvTitle = dialogView.findViewById<TextView>(R.id.tvLanTitle)
        val tvInstructions = dialogView.findViewById<TextView>(R.id.tvLanInstructions)
        val ivLanQr = dialogView.findViewById<ImageView>(R.id.ivLanQr)
        val tvLanUrl = dialogView.findViewById<TextView>(R.id.tvLanUrl)
        val tvLanPin = dialogView.findViewById<TextView>(R.id.tvLanPin)
        val tvLanStatus = dialogView.findViewById<TextView>(R.id.tvLanStatus)
        val btnClose = dialogView.findViewById<MaterialButton>(R.id.btnLanClose)

        tvTitle.text = getString(R.string.lan_sync_title)
        tvInstructions.text = getString(R.string.lan_sync_desc)

        val dialog = MaterialAlertDialogBuilder(this)
            .setView(dialogView)
            .setCancelable(true)
            .create()

        lanServer?.stop()
        lanServer = TvLanSyncServer(this) { importedConfig, count ->
            val nodes = ConfigParser.parse(importedConfig)
            if (nodes.isNotEmpty()) {
                val syncResult = settings.syncSubscriptionNodes(nodes)
                notifyNodesChanged()
                tvLanStatus.text = "✅ Получено ${syncResult.total} узлов!"
                tvLanStatus.setTextColor(ContextCompat.getColor(this, R.color.accent_mint))
                Toast.makeText(this, "Подписка успешно получена по локальной сети!", Toast.LENGTH_LONG).show()
            }
        }

        if (lanServer?.start() == true) {
            val syncUrl = lanServer!!.getSyncUrl()
            tvLanUrl.text = syncUrl
            tvLanPin.text = lanServer!!.pinCode

            val qrBitmap = QrUtils.generateQrBitmap(syncUrl, 500)
            ivLanQr.setImageBitmap(qrBitmap)
        } else {
            tvLanStatus.text = "Не удалось запустить локальный сервер Wi-Fi"
        }

        btnClose.setOnClickListener {
            dialog.dismiss()
        }

        dialog.setOnDismissListener {
            lanServer?.stop()
            lanServer = null
        }

        dialog.show()
    }

    fun showShareQrDialog(customNode: VpnNode? = null) {
        val nodeToShare = customNode ?: settings.getSelectedNode()
        val shareContent = nodeToShare?.rawUri?.takeIf { it.isNotEmpty() }
            ?: (if (customNode == null) settings.lastSubscriptionUrl else null)
            ?: "vless://${nodeToShare?.uuid}@${nodeToShare?.server}:${nodeToShare?.port}?security=${nodeToShare?.security}&sni=${nodeToShare?.sni}#${nodeToShare?.name}"

        val dialogView = LayoutInflater.from(this).inflate(R.layout.dialog_qr_display, null)
        val ivShareQr = dialogView.findViewById<ImageView>(R.id.ivShareQr)
        val tvQrTitle = dialogView.findViewById<TextView>(R.id.tvQrTitle)
        val tvQrDetails = dialogView.findViewById<TextView>(R.id.tvQrDetails)
        val btnClose = dialogView.findViewById<MaterialButton>(R.id.btnQrClose)

        if (customNode != null) {
            tvQrTitle.text = "Поделиться узлом: ${customNode.name}"
        }
        tvQrDetails.text = shareContent
        val qrBitmap = QrUtils.generateQrBitmap(shareContent, 500)
        ivShareQr.setImageBitmap(qrBitmap)

        val dialog = MaterialAlertDialogBuilder(this)
            .setView(dialogView)
            .setCancelable(true)
            .create()

        btnClose.setOnClickListener {
            dialog.dismiss()
        }

        dialog.show()
    }

    fun fetchSubscription(url: String, isManual: Boolean = true) {
        if (isManual) {
            Toast.makeText(this, "Синхронизация подписки…", Toast.LENGTH_SHORT).show()
        }
        settings.lastSubscriptionUrl = url

        Thread {
            try {
                val cleanUrl = if (url.contains("#")) url.substringBefore("#") else url
                val client = okhttp3.OkHttpClient()
                val request = okhttp3.Request.Builder()
                    .url(cleanUrl)
                    .header("User-Agent", "EvertyMultiLink/1.0 (v2rayng/clash-meta)")
                    .build()
                client.newCall(request).execute().use { response ->
                    val body = response.body?.string() ?: ""
                    val nodes = ConfigParser.parse(body)
                    mainHandler.post {
                        if (nodes.isNotEmpty()) {
                            val res = settings.syncSubscriptionNodes(nodes)
                            notifyNodesChanged()
                            val msg = "Подписка обновлена: +${res.added} новых, ${res.updated} обновлено, ${res.total} узлов"
                            Toast.makeText(this, msg, Toast.LENGTH_LONG).show()
                        } else if (isManual) {
                            Toast.makeText(this, "В подписке не найдено узлов", Toast.LENGTH_SHORT).show()
                        }
                    }
                }
            } catch (e: Exception) {
                if (isManual) {
                    mainHandler.post {
                        Toast.makeText(this, "Ошибка загрузки: ${e.message}", Toast.LENGTH_LONG).show()
                    }
                }
            }
        }.start()
    }

    fun notifyNodesChanged() {
        pagerAdapter.homeFragment.updateModeDisplay()
        pagerAdapter.serversFragment.refreshList()
    }

    private fun handleScannedQr(content: String) {
        val trimmed = content.trim()
        if (trimmed.startsWith("http://") && trimmed.contains("?pin=")) {
            val pin = trimmed.substringAfter("?pin=")
            val activeNode = settings.getSelectedNode()
            val sharePayload = activeNode?.rawUri ?: settings.lastSubscriptionUrl ?: ""

            if (sharePayload.isEmpty()) {
                Toast.makeText(this, "Нет активной подписки для передачи на ТВ", Toast.LENGTH_LONG).show()
                return
            }

            MaterialAlertDialogBuilder(this)
                .setTitle("📺 Найдено Everty TV")
                .setMessage("Перенести текущую подписку на экран телевизора?")
                .setPositiveButton("Передать") { _, _ ->
                    TvLanClient.sendConfigToTv(trimmed, pin, sharePayload) { _, msg ->
                        mainHandler.post {
                            Toast.makeText(this, msg, Toast.LENGTH_LONG).show()
                        }
                    }
                }
                .setNegativeButton("Отмена", null)
                .show()
        } else {
            if (trimmed.startsWith("http://") || trimmed.startsWith("https://")) {
                fetchSubscription(trimmed, isManual = true)
            } else {
                val nodes = ConfigParser.parse(trimmed)
                if (nodes.isNotEmpty()) {
                    settings.syncSubscriptionNodes(nodes)
                    notifyNodesChanged()
                    Toast.makeText(this, "Добавлено ${nodes.size} узлов", Toast.LENGTH_SHORT).show()
                } else {
                    Toast.makeText(this, "Не удалось распознать конфигурацию", Toast.LENGTH_LONG).show()
                }
            }
        }
    }

    private fun handleIntent(intent: Intent?) {
        val url = intent?.getStringExtra("import_url") ?: intent?.dataString
        if (!url.isNullOrEmpty()) {
            handleScannedQr(url)
        }
    }

    override fun onStart() {
        super.onStart()
        val filter = IntentFilter(EvertyVpnService.ACTION_STATE_CHANGED)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(vpnStateReceiver, filter, RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(vpnStateReceiver, filter)
        }
    }

    override fun onStop() {
        super.onStop()
        try {
            unregisterReceiver(vpnStateReceiver)
        } catch (_: Exception) {}
    }

    override fun onDestroy() {
        lanServer?.stop()
        lanServer = null
        super.onDestroy()
    }
}
