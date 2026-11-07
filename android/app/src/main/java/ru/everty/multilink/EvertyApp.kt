package ru.everty.multilink

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.os.Build

class EvertyApp : Application() {

    companion object {
        const val VPN_CHANNEL_ID = "everty_vpn_status"
        lateinit var instance: EvertyApp
            private set
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        createNotificationChannels()
    }

    private fun createNotificationChannels() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val name = "Everty MultiLink VPN"
            val descriptionText = "Уведомления о статусе активного VPN-подключения"
            val importance = NotificationManager.IMPORTANCE_LOW
            val channel = NotificationChannel(VPN_CHANNEL_ID, name, importance).apply {
                description = descriptionText
                setShowBadge(false)
            }
            val notificationManager = getSystemService(NotificationManager::class.java)
            notificationManager?.createNotificationChannel(channel)
        }
    }
}
