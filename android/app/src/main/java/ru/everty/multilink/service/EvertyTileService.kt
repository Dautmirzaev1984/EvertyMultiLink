package ru.everty.multilink.service

import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.graphics.drawable.Icon
import android.net.VpnService
import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import androidx.annotation.RequiresApi
import ru.everty.multilink.R
import ru.everty.multilink.model.AppSettings
import ru.everty.multilink.ui.MainActivity

@RequiresApi(Build.VERSION_CODES.N)
class EvertyTileService : TileService() {

    private val stateReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            updateTileState()
        }
    }

    override fun onStartListening() {
        super.onStartListening()
        val filter = IntentFilter(EvertyVpnService.ACTION_STATE_CHANGED)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(stateReceiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(stateReceiver, filter)
        }
        updateTileState()
    }

    override fun onStopListening() {
        super.onStopListening()
        try {
            unregisterReceiver(stateReceiver)
        } catch (_: Exception) {}
    }

    override fun onClick() {
        super.onClick()
        val isConnected = (EvertyVpnService.currentState == EvertyVpnService.STATE_CONNECTED)

        if (isConnected) {
            val intent = Intent(this, EvertyVpnService::class.java).apply {
                action = EvertyVpnService.ACTION_DISCONNECT
            }
            startService(intent)
        } else {
            val prepareIntent = VpnService.prepare(this)
            if (prepareIntent != null) {
                val mainIntent = Intent(this, MainActivity::class.java).apply {
                    flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
                    putExtra("auto_connect", true)
                }
                if (Build.VERSION.SDK_INT >= 34) {
                    val pendingIntent = PendingIntent.getActivity(
                        this, 0, mainIntent,
                        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
                    )
                    startActivityAndCollapse(pendingIntent)
                } else {
                    @Suppress("DEPRECATION")
                    startActivityAndCollapse(mainIntent)
                }
            } else {
                val intent = Intent(this, EvertyVpnService::class.java).apply {
                    action = EvertyVpnService.ACTION_CONNECT
                }
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    startForegroundService(intent)
                } else {
                    startService(intent)
                }
            }
        }
        updateTileState()
    }

    private fun updateTileState() {
        val tile = qsTile ?: return
        val state = EvertyVpnService.currentState
        val settings = AppSettings(this)
        val forced = settings.getForcedNode()

        when (state) {
            EvertyVpnService.STATE_CONNECTED -> {
                tile.state = Tile.STATE_ACTIVE
                tile.label = "Everty MultiLink"
                val subtitleText = forced?.name ?: "Multi-Link (${settings.policy.label})"
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    tile.subtitle = subtitleText
                }
                tile.icon = Icon.createWithResource(this, R.drawable.ic_qs_everty)
            }
            EvertyVpnService.STATE_CONNECTING -> {
                tile.state = Tile.STATE_INACTIVE
                tile.label = "Everty MultiLink"
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    tile.subtitle = "Подключение…"
                }
                tile.icon = Icon.createWithResource(this, R.drawable.ic_qs_everty)
            }
            else -> {
                tile.state = Tile.STATE_INACTIVE
                tile.label = "Everty MultiLink"
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    tile.subtitle = "Отключено"
                }
                tile.icon = Icon.createWithResource(this, R.drawable.ic_qs_everty)
            }
        }
        tile.updateTile()
    }
}
