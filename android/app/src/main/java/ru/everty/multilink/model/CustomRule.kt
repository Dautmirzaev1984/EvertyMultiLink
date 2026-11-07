package ru.everty.multilink.model

import java.util.UUID

data class CustomRule(
    val id: String = UUID.randomUUID().toString(),
    val pattern: String,
    val type: String = "DOMAIN-KEYWORD", // DOMAIN-SUFFIX, DOMAIN-KEYWORD, DOMAIN, IP-CIDR
    val action: String = "DIRECT"        // DIRECT, PROXY
) {
    fun toMihomoRule(proxyGroupName: String = "MULTI_LINK"): String {
        val target = if (action == "DIRECT") "DIRECT" else proxyGroupName
        return "$type,$pattern,$target"
    }
}
