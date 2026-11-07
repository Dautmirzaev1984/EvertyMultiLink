package ru.everty.multilink.service

import android.app.Notification
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.Build
import android.os.ParcelFileDescriptor
import android.util.Log
import androidx.core.app.NotificationCompat
import ru.everty.multilink.EvertyApp
import ru.everty.multilink.R
import ru.everty.multilink.core.MihomoConfigBuilder
import ru.everty.multilink.model.AppSettings
import ru.everty.multilink.ui.MainActivity
import java.io.BufferedReader
import java.io.File
import java.io.InputStreamReader
import java.util.concurrent.atomic.AtomicBoolean

class EvertyVpnService : VpnService() {

    companion object {
        private const val TAG = "EvertyVpnService"
        const val ACTION_CONNECT = "ru.everty.multilink.CONNECT"
        const val ACTION_DISCONNECT = "ru.everty.multilink.DISCONNECT"
        const val ACTION_STATE_CHANGED = "ru.everty.multilink.VPN_STATE_CHANGED"
        const val EXTRA_STATE = "vpn_state"
        const val STATE_DISCONNECTED = 0
        const val STATE_CONNECTING = 1
        const val STATE_CONNECTED = 2

        var currentState = STATE_DISCONNECTED
            private set
    }

    private var vpnInterface: ParcelFileDescriptor? = null
    private val isRunning = AtomicBoolean(false)
    private var workerThread: Thread? = null
    private var mihomoProcess: Process? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_DISCONNECT -> {
                disconnect()
                stopSelf()
            }
            ACTION_CONNECT -> {
                connect()
            }
        }
        return START_NOT_STICKY
    }

    private fun connect() {
        val settings = AppSettings(this)
        val selectedNode = settings.getSelectedNode()
        if (selectedNode == null || !selectedNode.isEnabled) {
            Log.w(TAG, "Cannot connect: no active node selected")
            broadcastState(STATE_DISCONNECTED)
            stopSelf()
            return
        }

        broadcastState(STATE_CONNECTING)
        startForeground(1001, buildNotification("Подключение к ${selectedNode.name}…", false))

        workerThread = Thread {
            try {
                // 1. Prepare Mihomo configuration
                val mihomoDir = File(filesDir, "mihomo").apply { mkdirs() }
                val configFile = MihomoConfigBuilder.generateConfig(
                    outputDir = mihomoDir,
                    settings = settings
                )
                Log.d(TAG, "Generated Mihomo config at ${configFile.absolutePath}")

                // 2. Locate native binaries
                val nativeLibDir = applicationInfo.nativeLibraryDir
                val mihomoBin = File(nativeLibDir, "libmihomo.so")
                val actualMihomo = if (mihomoBin.exists()) mihomoBin.absolutePath else "/data/local/tmp/mihomo"

                // 3. Start Mihomo Core
                Log.d(TAG, "Starting Mihomo process: $actualMihomo -d ${mihomoDir.absolutePath}")
                val mihomoPb = ProcessBuilder(actualMihomo, "-d", mihomoDir.absolutePath)
                    .redirectErrorStream(true)
                val proc = mihomoPb.start()
                mihomoProcess = proc

                // Pipe Mihomo logs to Android logcat
                Thread {
                    try {
                        BufferedReader(InputStreamReader(proc.inputStream)).use { reader ->
                            var line: String?
                            while (reader.readLine().also { line = it } != null) {
                                Log.d("MihomoCore", line ?: "")
                            }
                        }
                    } catch (_: Exception) {}
                }.start()

                // Wait for Mihomo to bind 127.0.0.1:7890
                Thread.sleep(600)

                // 4. Establish Android VPN TUN interface
                val sessionTitle = settings.getForcedNode()?.name ?: "MultiLink: ${settings.policy.label}"
                val builder = Builder()
                    .setSession("Everty: $sessionTitle")
                    .addAddress("172.19.0.1", 30)
                    .addDnsServer("1.1.1.1")
                    .addDnsServer("8.8.8.8")
                    .setMtu(1500)
                    .addRoute("0.0.0.0", 0)

                // Per-App Split Tunneling or default self-exclusion
                if (settings.isPerAppVpnEnabled && settings.perAppVpnPackages.isNotEmpty()) {
                    if (settings.perAppVpnMode == "INCLUDE") {
                        for (pkg in settings.perAppVpnPackages) {
                            try {
                                builder.addAllowedApplication(pkg)
                            } catch (e: Exception) {
                                Log.w(TAG, "addAllowedApplication failed for $pkg: ${e.message}")
                            }
                        }
                    } else {
                        for (pkg in settings.perAppVpnPackages) {
                            try {
                                builder.addDisallowedApplication(pkg)
                            } catch (e: Exception) {
                                Log.w(TAG, "addDisallowedApplication failed for $pkg: ${e.message}")
                            }
                        }
                        try {
                            builder.addDisallowedApplication(packageName)
                        } catch (_: Exception) {}
                    }
                } else {
                    // Prevent VPN loopback: Disallow this app so Mihomo's proxy traffic goes out directly
                    try {
                        builder.addDisallowedApplication(packageName)
                    } catch (e: Exception) {
                        Log.w(TAG, "addDisallowedApplication failed: ${e.message}")
                    }
                }

                vpnInterface = builder.establish()

                if (vpnInterface != null) {
                    val pfd = vpnInterface!!
                    val fd = pfd.fd
                    isRunning.set(true)

                    Log.d(TAG, "VPN TUN established successfully (fd=$fd), starting Tun2Socks...")

                    // 5. Start Tun2Socks in-process via JNI
                    Tun2Socks.start(fd, "socks5://127.0.0.1:7890", 1500)

                    broadcastState(STATE_CONNECTED)
                    startForeground(1001, buildNotification("Защищено через ${selectedNode.name}", true))
                    Log.d(TAG, "VPN Connected and Tun2Socks running.")

                    // Keep running until disconnected or service stopped
                    while (isRunning.get()) {
                        try {
                            Thread.sleep(1000)
                        } catch (_: InterruptedException) {
                            break
                        }
                    }
                } else {
                    Log.e(TAG, "Failed to establish VPN interface (null)")
                    broadcastState(STATE_DISCONNECTED)
                    stopSelf()
                }
            } catch (e: InterruptedException) {
                // Service intentionally stopped
            } catch (e: Exception) {
                Log.e(TAG, "Error starting VPN: ${e.message}", e)
                broadcastState(STATE_DISCONNECTED)
                stopSelf()
            }
        }.apply { start() }
    }

    private fun disconnect() {
        Log.d(TAG, "Disconnecting VPN...")
        isRunning.set(false)
        workerThread?.interrupt()
        workerThread = null

        try {
            Tun2Socks.stop()
        } catch (e: Exception) {
            Log.e(TAG, "Error stopping Tun2Socks: ${e.message}")
        }

        try {
            mihomoProcess?.destroy()
        } catch (_: Exception) {}
        mihomoProcess = null

        try {
            vpnInterface?.close()
        } catch (_: Exception) {}
        vpnInterface = null

        broadcastState(STATE_DISCONNECTED)
        stopForeground(STOP_FOREGROUND_REMOVE)
        Log.d(TAG, "VPN Disconnected.")
    }

    private fun broadcastState(state: Int) {
        currentState = state
        val intent = Intent(ACTION_STATE_CHANGED).apply {
            putExtra(EXTRA_STATE, state)
            setPackage(packageName)
        }
        sendBroadcast(intent)
    }

    private fun buildNotification(text: String, isConnected: Boolean): Notification {
        val launchIntent = Intent(this, MainActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            this, 0, launchIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val disconnectIntent = Intent(this, EvertyVpnService::class.java).apply {
            action = ACTION_DISCONNECT
        }
        val disconnectPending = PendingIntent.getService(
            this, 1, disconnectIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val builder = NotificationCompat.Builder(this, EvertyApp.VPN_CHANNEL_ID)
            .setContentTitle("Everty MultiLink")
            .setContentText(text)
            .setSmallIcon(R.mipmap.ic_launcher)
            .setContentIntent(pendingIntent)
            .setOngoing(isConnected)
            .setPriority(NotificationCompat.PRIORITY_LOW)

        if (isConnected) {
            builder.addAction(android.R.drawable.ic_menu_close_clear_cancel, "Отключить", disconnectPending)
        }

        return builder.build()
    }

    override fun onDestroy() {
        disconnect()
        super.onDestroy()
    }
}
