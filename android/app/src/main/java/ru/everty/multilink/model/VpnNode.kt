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
    var nodesCount: Int = 0
)
