package ru.everty.multilink.model

import android.content.Context
import android.content.SharedPreferences
import com.google.gson.Gson
import com.google.gson.reflect.TypeToken

data class SyncResult(
    val added: Int,
    val updated: Int,
    val removed: Int,
    val total: Int
)

class AppSettings(context: Context) {

    private val prefs: SharedPreferences = context.getSharedPreferences("everty_settings", Context.MODE_PRIVATE)
    private val gson = Gson()

    companion object {
        private const val KEY_NODES = "vpn_nodes"
        private const val KEY_SELECTED_NODE_ID = "selected_node_id"
        private const val KEY_FORCED_NODE_ID = "forced_node_id"
        private const val KEY_POLICY = "routing_policy"
        private const val KEY_SPLIT_RU = "split_ru_direct"
        private const val KEY_SUBSCRIPTION_URL = "last_subscription_url"
        private const val KEY_LAST_SUB_UPDATE = "last_sub_update_time"
        private const val KEY_AUTO_UPDATE_ON_START = "auto_update_on_start"
        private const val KEY_CUSTOM_RULES = "custom_routing_rules"
        private const val KEY_PER_APP_ENABLED = "per_app_vpn_enabled"
        private const val KEY_PER_APP_PACKAGES = "per_app_vpn_packages"
        private const val KEY_PER_APP_MODE = "per_app_vpn_mode"
    }

    var selectedNodeId: String?
        get() = prefs.getString(KEY_SELECTED_NODE_ID, null)
        set(value) = prefs.edit().putString(KEY_SELECTED_NODE_ID, value).apply()

    var forcedNodeId: String?
        get() = prefs.getString(KEY_FORCED_NODE_ID, null)
        set(value) = prefs.edit().putString(KEY_FORCED_NODE_ID, value).apply()

    var policy: Policy
        get() {
            val name = prefs.getString(KEY_POLICY, Policy.SMART.name) ?: Policy.SMART.name
            return try { Policy.valueOf(name) } catch (e: Exception) { Policy.SMART }
        }
        set(value) = prefs.edit().putString(KEY_POLICY, value.name).apply()

    var isSplitRuEnabled: Boolean
        get() = prefs.getBoolean(KEY_SPLIT_RU, true)
        set(value) = prefs.edit().putBoolean(KEY_SPLIT_RU, value).apply()

    var lastSubscriptionUrl: String?
        get() = prefs.getString(KEY_SUBSCRIPTION_URL, null)
        set(value) = prefs.edit().putString(KEY_SUBSCRIPTION_URL, value).apply()

    var lastSubscriptionUpdateTime: Long
        get() = prefs.getLong(KEY_LAST_SUB_UPDATE, 0L)
        set(value) = prefs.edit().putLong(KEY_LAST_SUB_UPDATE, value).apply()

    var isAutoUpdateOnStartEnabled: Boolean
        get() = prefs.getBoolean(KEY_AUTO_UPDATE_ON_START, true)
        set(value) = prefs.edit().putBoolean(KEY_AUTO_UPDATE_ON_START, value).apply()

    var isPerAppVpnEnabled: Boolean
        get() = prefs.getBoolean(KEY_PER_APP_ENABLED, false)
        set(value) = prefs.edit().putBoolean(KEY_PER_APP_ENABLED, value).apply()

    var perAppVpnMode: String
        get() = prefs.getString(KEY_PER_APP_MODE, "INCLUDE") ?: "INCLUDE"
        set(value) = prefs.edit().putString(KEY_PER_APP_MODE, value).apply()

    var perAppVpnPackages: Set<String>
        get() = prefs.getStringSet(KEY_PER_APP_PACKAGES, emptySet()) ?: emptySet()
        set(value) = prefs.edit().putStringSet(KEY_PER_APP_PACKAGES, value).apply()

    fun toggleAppPackage(packageName: String) {
        val current = perAppVpnPackages.toMutableSet()
        if (current.contains(packageName)) {
            current.remove(packageName)
        } else {
            current.add(packageName)
        }
        perAppVpnPackages = current
    }

    fun getNodes(): List<VpnNode> {
        val json = prefs.getString(KEY_NODES, null) ?: return emptyList()
        val type = object : TypeToken<List<VpnNode>>() {}.type
        return try {
            gson.fromJson(json, type) ?: emptyList()
        } catch (e: Exception) {
            emptyList()
        }
    }

    fun saveNodes(nodes: List<VpnNode>) {
        val json = gson.toJson(nodes)
        prefs.edit().putString(KEY_NODES, json).apply()
    }

    fun getSelectedNode(): VpnNode? {
        val nodes = getNodes()
        val forced = forcedNodeId?.let { fid -> nodes.find { it.id == fid && it.isEnabled } }
        if (forced != null) return forced

        val id = selectedNodeId ?: return nodes.firstOrNull { it.isEnabled }
        return nodes.find { it.id == id && it.isEnabled } ?: nodes.firstOrNull { it.isEnabled }
    }

    fun getForcedNode(): VpnNode? {
        val fid = forcedNodeId ?: return null
        return getNodes().find { it.id == fid }
    }

    fun selectNode(nodeId: String) {
        selectedNodeId = nodeId
        val nodes = getNodes()
        val updated = nodes.map { it.copy(isSelected = (it.id == nodeId)) }
        saveNodes(updated)
    }

    /**
     * Smart sync of subscription nodes:
     * - Updates changed names, ports, IPs, Reality keys
     * - Adds new nodes
     * - Removes obsolete nodes
     * - Preserves user preferences (disabled state, pinned state)
     */
    fun syncSubscriptionNodes(newNodes: List<VpnNode>): SyncResult {
        val current = getNodes()
        var added = 0
        var updated = 0

        val merged = mutableListOf<VpnNode>()
        val newSignatures = mutableSetOf<String>()

        for (n in newNodes) {
            val sig = "${n.server}:${n.port}:${n.uuid}"
            newSignatures.add(sig)

            val existing = current.find { "${it.server}:${it.port}:${it.uuid}" == sig }
            if (existing != null) {
                // Update properties but keep user settings
                val nodeUpdated = existing.copy(
                    name = n.name,
                    security = n.security,
                    sni = n.sni,
                    pbk = n.pbk,
                    sid = n.sid,
                    flow = n.flow,
                    type = n.type,
                    path = n.path,
                    rawUri = n.rawUri
                )
                if (existing.name != n.name || existing.sni != n.sni || existing.pbk != n.pbk) {
                    updated++
                }
                merged.add(nodeUpdated)
            } else {
                merged.add(n)
                added++
            }
        }

        // Count removed nodes that came from subscription
        val removed = current.count { "${it.server}:${it.port}:${it.uuid}" !in newSignatures }

        // If forced node was removed, reset pin
        if (forcedNodeId != null && merged.none { it.id == forcedNodeId }) {
            forcedNodeId = null
        }

        saveNodes(merged)
        lastSubscriptionUpdateTime = System.currentTimeMillis()

        return SyncResult(added = added, updated = updated, removed = removed, total = merged.size)
    }

    fun addOrUpdateNodes(newNodes: List<VpnNode>) {
        syncSubscriptionNodes(newNodes)
    }

    fun getCustomRules(): List<CustomRule> {
        val json = prefs.getString(KEY_CUSTOM_RULES, null)
        if (json.isNullOrEmpty()) {
            return listOf(
                CustomRule(pattern = "2ip.ru", type = "DOMAIN-KEYWORD", action = "DIRECT"),
                CustomRule(pattern = "speedtest.net", type = "DOMAIN-KEYWORD", action = "DIRECT")
            )
        }
        val type = object : TypeToken<List<CustomRule>>() {}.type
        return try {
            gson.fromJson(json, type) ?: emptyList()
        } catch (_: Exception) {
            emptyList()
        }
    }

    fun saveCustomRules(rules: List<CustomRule>) {
        val json = gson.toJson(rules)
        prefs.edit().putString(KEY_CUSTOM_RULES, json).apply()
    }

    fun addCustomRule(pattern: String, type: String = "DOMAIN-KEYWORD", action: String = "DIRECT") {
        val current = getCustomRules().toMutableList()
        current.add(0, CustomRule(pattern = pattern.trim(), type = type, action = action))
        saveCustomRules(current)
    }

    fun deleteCustomRule(ruleId: String) {
        val current = getCustomRules().filter { it.id != ruleId }
        saveCustomRules(current)
    }
}
