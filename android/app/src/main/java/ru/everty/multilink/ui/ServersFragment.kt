package ru.everty.multilink.ui

import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.TextView
import android.widget.Toast
import androidx.core.content.ContextCompat
import androidx.fragment.app.Fragment
import androidx.recyclerview.widget.LinearLayoutManager
import com.google.android.material.button.MaterialButton
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import ru.everty.multilink.R
import ru.everty.multilink.parser.ConfigParser
import ru.everty.multilink.databinding.FragmentServersBinding
import ru.everty.multilink.model.AppSettings
import ru.everty.multilink.model.VpnNode
import java.net.InetSocketAddress
import java.net.Socket
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

class ServersFragment : Fragment() {

    private var _binding: FragmentServersBinding? = null
    private val binding get() = _binding!!

    private lateinit var settings: AppSettings
    private lateinit var adapter: NodesAdapter

    override fun onCreateView(
        inflater: LayoutInflater, container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View {
        _binding = FragmentServersBinding.inflate(inflater, container, false)
        return binding.root
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        settings = AppSettings(requireContext())

        setupRecyclerView()
        setupListeners()
        refreshList()
    }

    private fun setupRecyclerView() {
        adapter = NodesAdapter(
            nodes = emptyList(),
            forcedNodeId = settings.forcedNodeId,
            onNodeClicked = { node ->
                if (settings.forcedNodeId == node.id) {
                    settings.forcedNodeId = null
                    Toast.makeText(requireContext(), "Фиксация снята: активен Multi-Link", Toast.LENGTH_SHORT).show()
                } else {
                    settings.forcedNodeId = node.id
                    settings.selectNode(node.id)
                    Toast.makeText(requireContext(), "Сервер зафиксирован: ${node.name}", Toast.LENGTH_SHORT).show()
                }
                refreshList()
                (activity as? MainActivity)?.notifyNodesChanged()
            },
            onNodeLongClick = { node ->
                showNodeActionDialog(node)
            }
        )

        binding.rvNodes.layoutManager = LinearLayoutManager(requireContext())
        binding.rvNodes.adapter = adapter
    }

    private fun setupListeners() {
        binding.btnUpdateSub.setOnClickListener {
            val url = settings.lastSubscriptionUrl
            if (!url.isNullOrBlank()) {
                (activity as? MainActivity)?.fetchSubscription(url, isManual = true)
            } else {
                showAddSubscriptionDialog()
            }
        }

        binding.btnAddSub.setOnClickListener {
            showAddSubscriptionDialog()
        }
    }

    fun refreshList() {
        if (_binding == null) return

        val nodes = settings.getNodes()
        val enabledCount = nodes.count { it.isEnabled }
        binding.tvSubscriptionCount.text = "$enabledCount / ${nodes.size} узлов"
        binding.tvSubscriptionUrl.text = settings.lastSubscriptionUrl ?: "Подписка добавлена вручную"

        val lastUpdate = settings.lastSubscriptionUpdateTime
        if (lastUpdate > 0) {
            val sdf = SimpleDateFormat("dd.MM HH:mm:ss", Locale.getDefault())
            binding.tvSubscriptionUpdated.text = "Синхронизировано: ${sdf.format(Date(lastUpdate))}"
        } else {
            binding.tvSubscriptionUpdated.text = "Синхронизация не выполнялась"
        }

        binding.tvNodesHeader.text = "Узлы для Multi-Link (${nodes.size})"
        adapter.updateNodes(nodes, settings.forcedNodeId)

        // Run latency pings in background
        measureLatencies(nodes)
    }

    private fun measureLatencies(nodes: List<VpnNode>) {
        Thread {
            val uiHandler = Handler(Looper.getMainLooper())
            val nodesToTest = nodes.filter { it.isEnabled }.take(15)

            for (node in nodesToTest) {
                val ping = testTcpPing(node.server, node.port)
                if (ping > 0) {
                    node.pingMs = ping
                    uiHandler.post {
                        if (_binding != null) {
                            adapter.notifyDataSetChanged()
                        }
                    }
                }
            }
        }.start()
    }

    private fun testTcpPing(host: String, port: Int): Long {
        return try {
            val start = System.currentTimeMillis()
            Socket().use { socket ->
                socket.connect(InetSocketAddress(host, port), 2500)
            }
            System.currentTimeMillis() - start
        } catch (_: Exception) {
            -1L
        }
    }

    private fun showAddSubscriptionDialog() {
        val input = EditText(requireContext()).apply {
            hint = "https://... или vless://..."
            setTextColor(ContextCompat.getColor(context, R.color.text_primary))
            setHintTextColor(ContextCompat.getColor(context, R.color.text_muted))
            background = ContextCompat.getDrawable(context, R.drawable.card_background)
            setPadding(30, 30, 30, 30)
        }

        MaterialAlertDialogBuilder(requireContext())
            .setTitle("Добавить подписку или узел")
            .setMessage("Вставьте URL подписки VLESS или ссылку vless://:")
            .setView(input)
            .setPositiveButton("Импортировать") { _, _ ->
                val text = input.text.toString().trim()
                if (text.startsWith("http://") || text.startsWith("https://")) {
                    (activity as? MainActivity)?.fetchSubscription(text, isManual = true)
                } else if (text.startsWith("vless://")) {
                    val nodes = ConfigParser.parse(text)
                    if (nodes.isNotEmpty()) {
                        settings.syncSubscriptionNodes(nodes)
                        refreshList()
                        (activity as? MainActivity)?.notifyNodesChanged()
                        Toast.makeText(requireContext(), "Добавлено ${nodes.size} узлов", Toast.LENGTH_SHORT).show()
                    } else {
                        Toast.makeText(requireContext(), "Неверный формат VLESS", Toast.LENGTH_SHORT).show()
                    }
                }
            }
            .setNegativeButton("Отмена", null)
            .show()
    }

    private fun showNodeActionDialog(node: VpnNode) {
        val dialogView = LayoutInflater.from(requireContext()).inflate(R.layout.dialog_node_actions, null)
        val tvTitle = dialogView.findViewById<TextView>(R.id.tvActionNodeTitle)
        val tvDetails = dialogView.findViewById<TextView>(R.id.tvActionNodeDetails)
        val btnPin = dialogView.findViewById<MaterialButton>(R.id.btnPinNode)
        val btnToggle = dialogView.findViewById<MaterialButton>(R.id.btnToggleNodeActive)
        val btnShare = dialogView.findViewById<MaterialButton>(R.id.btnShareNodeQr)
        val btnDelete = dialogView.findViewById<MaterialButton>(R.id.btnDeleteNode)
        val btnCancel = dialogView.findViewById<MaterialButton>(R.id.btnCancelNodeAction)

        tvTitle.text = node.name
        tvDetails.text = node.getDisplayDetails()

        val isPinned = (settings.forcedNodeId == node.id)
        if (isPinned) {
            btnPin.text = "🔄 Снять принудительную фиксацию (Вернуть Multi-Link)"
            btnPin.setTextColor(ContextCompat.getColor(requireContext(), R.color.accent_gold))
        } else {
            btnPin.text = "📌 Принудительно зафиксировать этот сервер"
            btnPin.setTextColor(ContextCompat.getColor(requireContext(), R.color.accent_mint))
        }

        if (node.isEnabled) {
            btnToggle.text = "⏸️ Сделать неактивным (Выключить)"
            btnToggle.setTextColor(ContextCompat.getColor(requireContext(), R.color.accent_gold))
        } else {
            btnToggle.text = "▶️ Сделать активным (Включить)"
            btnToggle.setTextColor(ContextCompat.getColor(requireContext(), R.color.accent_mint))
        }

        val dialog = MaterialAlertDialogBuilder(requireContext())
            .setView(dialogView)
            .create()

        btnPin.setOnClickListener {
            dialog.dismiss()
            if (isPinned) {
                settings.forcedNodeId = null
                Toast.makeText(requireContext(), "Фиксация снята: Multi-Link активен", Toast.LENGTH_SHORT).show()
            } else {
                settings.forcedNodeId = node.id
                settings.selectNode(node.id)
                Toast.makeText(requireContext(), "Сервер зафиксирован: ${node.name}", Toast.LENGTH_SHORT).show()
            }
            refreshList()
            (activity as? MainActivity)?.notifyNodesChanged()
        }

        btnToggle.setOnClickListener {
            val list = settings.getNodes().toMutableList()
            val idx = list.indexOfFirst { it.id == node.id }
            if (idx >= 0) {
                list[idx] = list[idx].copy(isEnabled = !node.isEnabled)
                if (list[idx].isSelected && !list[idx].isEnabled) {
                    list[idx] = list[idx].copy(isSelected = false)
                    val next = list.firstOrNull { it.isEnabled }
                    if (next != null) {
                        val nextIdx = list.indexOfFirst { it.id == next.id }
                        list[nextIdx] = list[nextIdx].copy(isSelected = true)
                        settings.selectedNodeId = next.id
                    }
                }
                if (settings.forcedNodeId == node.id && !list[idx].isEnabled) {
                    settings.forcedNodeId = null
                }
                settings.saveNodes(list)
                refreshList()
                (activity as? MainActivity)?.notifyNodesChanged()
            }
            dialog.dismiss()
        }

        btnShare.setOnClickListener {
            dialog.dismiss()
            (activity as? MainActivity)?.showShareQrDialog(node)
        }

        btnDelete.setOnClickListener {
            dialog.dismiss()
            MaterialAlertDialogBuilder(requireContext())
                .setTitle("Удалить сервер?")
                .setMessage("Вы действительно хотите удалить ${node.name}?")
                .setPositiveButton("Удалить") { _, _ ->
                    val list = settings.getNodes().toMutableList()
                    list.removeAll { it.id == node.id }
                    if (settings.forcedNodeId == node.id) {
                        settings.forcedNodeId = null
                    }
                    if (list.none { it.isSelected } && list.isNotEmpty()) {
                        list[0] = list[0].copy(isSelected = true)
                        settings.selectedNodeId = list[0].id
                    }
                    settings.saveNodes(list)
                    refreshList()
                    (activity as? MainActivity)?.notifyNodesChanged()
                }
                .setNegativeButton("Отмена", null)
                .show()
        }

        btnCancel.setOnClickListener {
            dialog.dismiss()
        }

        dialog.show()
    }

    override fun onResume() {
        super.onResume()
        refreshList()
    }

    override fun onDestroyView() {
        super.onDestroyView()
        _binding = null
    }
}
