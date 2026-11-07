package ru.everty.multilink.model

data class VpnNode(
    val id: String,
    var name: String,
    val server: String,
    val port: Int,
    val uuid: String,
    val security: String = "reality",
    val sni: String = "",
    val pbk: String = "",
    val sid: String = "",
    val spx: String = "",
    val flow: String = "",
    val type: String = "tcp",
    val path: String = "",
    var pingMs: Long = -1,
    var isEnabled: Boolean = true,
    var isSelected: Boolean = false,
    val rawUri: String = "",
    var subscriptionId: String? = null
) {
    fun getDisplayDetails(): String {
        return "${security.uppercase()} • ${type.uppercase()} • $server:$port"
    }
}

enum class Policy(val label: String) {
    SMART("Умный выбор"),
    LATENCY("Быстрый (Мин. RTT)"),
    BALANCE("Сбалансированный"),
    STABLE("Резервный (Fallback)")
}

data class Subscription(
    val id: String,
    var name: String,
    var url: String,
    var updatedAt: Long = 0L,
    var nodesCount: Int = 0,
    var expireTimestamp: Long = 0L,
    var uploadBytes: Long = 0L,
    var downloadBytes: Long = 0L,
    var totalBytes: Long = 0L
) {
    fun getRemainingDaysText(): String? {
        if (expireTimestamp <= 0) return null
        val nowSec = System.currentTimeMillis() / 1000
        val diffSec = expireTimestamp - nowSec
        if (diffSec <= 0) return "⚠️ Истекла"
        val days = (diffSec / 86400).toInt()
        val suffix = when {
            days % 100 in 11..19 -> "дней"
            days % 10 == 1 -> "день"
            days % 10 in 2..4 -> "дня"
            else -> "дней"
        }
        return if (days == 0) "⏳ < 1 дня" else "⏳ $days $suffix"
    }

    fun getExpireFormattedDate(): String? {
        if (expireTimestamp <= 0) return null
        val sdf = java.text.SimpleDateFormat("dd.MM.yyyy", java.util.Locale.getDefault())
        return sdf.format(java.util.Date(expireTimestamp * 1000))
    }
}
