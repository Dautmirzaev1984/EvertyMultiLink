package ru.everty.multilink.ui

import android.content.Intent
import android.content.pm.ApplicationInfo
import android.content.pm.PackageManager
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ArrayAdapter
import android.widget.EditText
import android.widget.Toast
import androidx.core.widget.doAfterTextChanged
import androidx.fragment.app.Fragment
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.google.android.material.button.MaterialButton
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import ru.everty.multilink.R
import ru.everty.multilink.databinding.FragmentRoutingBinding
import ru.everty.multilink.model.AppSettings

class RoutingFragment : Fragment() {

    private var _binding: FragmentRoutingBinding? = null
    private val binding get() = _binding!!

    private lateinit var settings: AppSettings
    private lateinit var rulesAdapter: CustomRulesAdapter

    override fun onCreateView(
        inflater: LayoutInflater, container: ViewGroup?,
        savedInstanceState: Bundle?
    ): View {
        _binding = FragmentRoutingBinding.inflate(inflater, container, false)
        return binding.root
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)
        settings = AppSettings(requireContext())

        setupSplitRu()
        setupCustomRules()
        setupPerAppVpn()
        setupAutoUpdateAndGithub()
    }

    private fun setupAutoUpdateAndGithub() {
        binding.switchAutoUpdateSub.isChecked = settings.isAutoUpdateOnStartEnabled
        binding.switchAutoUpdateSub.setOnCheckedChangeListener { _, isChecked ->
            settings.isAutoUpdateOnStartEnabled = isChecked
        }

        binding.cardGithubLink.setOnClickListener {
            try {
                val browserIntent = Intent(Intent.ACTION_VIEW, android.net.Uri.parse("https://github.com/vaalimusic/EvertyMultiLink"))
                startActivity(browserIntent)
            } catch (_: Exception) {
                Toast.makeText(requireContext(), "Не удалось открыть ссылку", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun setupSplitRu() {
        binding.switchSplitRu.isChecked = settings.isSplitRuEnabled
        binding.switchSplitRu.setOnCheckedChangeListener { _, isChecked ->
            settings.isSplitRuEnabled = isChecked
            Toast.makeText(
                requireContext(),
                if (isChecked) "Прямой доступ для РФ включен" else "Прямой доступ для РФ выключен",
                Toast.LENGTH_SHORT
            ).show()
        }
    }

    private fun setupCustomRules() {
        val actions = listOf("DIRECT (Напрямую)", "PROXY (Через VPN)")
        val spinnerAdapter = ArrayAdapter(requireContext(), android.R.layout.simple_spinner_dropdown_item, actions)
        binding.spinnerRuleAction.adapter = spinnerAdapter

        rulesAdapter = CustomRulesAdapter(
            rules = settings.getCustomRules(),
            onDeleteClick = { rule ->
                settings.deleteCustomRule(rule.id)
                refreshRulesList()
            }
        )

        binding.rvCustomRules.layoutManager = LinearLayoutManager(requireContext())
        binding.rvCustomRules.adapter = rulesAdapter

        binding.btnAddRule.setOnClickListener {
            val text = binding.etNewRulePattern.text.toString().trim()
            if (text.isEmpty()) {
                binding.etNewRulePattern.error = "Введите домен или IP"
                return@setOnClickListener
            }

            val isDirect = binding.spinnerRuleAction.selectedItemPosition == 0
            val action = if (isDirect) "DIRECT" else "PROXY"
            val type = when {
                text.contains("/") -> "IP-CIDR"
                text.startsWith(".") -> "DOMAIN-SUFFIX"
                text.contains(".") -> "DOMAIN-KEYWORD"
                else -> "DOMAIN-KEYWORD"
            }

            settings.addCustomRule(text, type, action)
            binding.etNewRulePattern.text.clear()
            refreshRulesList()
            Toast.makeText(requireContext(), "Правило для $text добавлено", Toast.LENGTH_SHORT).show()
        }
    }

    private fun refreshRulesList() {
        rulesAdapter.updateRules(settings.getCustomRules())
    }

    private fun setupPerAppVpn() {
        binding.switchPerAppVpn.isChecked = settings.isPerAppVpnEnabled
        binding.layoutPerAppSettings.visibility = if (settings.isPerAppVpnEnabled) View.VISIBLE else View.GONE

        binding.switchPerAppVpn.setOnCheckedChangeListener { _, isChecked ->
            settings.isPerAppVpnEnabled = isChecked
            binding.layoutPerAppSettings.visibility = if (isChecked) View.VISIBLE else View.GONE
        }

        if (settings.perAppVpnMode == "EXCLUDE") {
            binding.rbPerAppExclude.isChecked = true
        } else {
            binding.rbPerAppInclude.isChecked = true
        }

        binding.rgPerAppMode.setOnCheckedChangeListener { _, checkedId ->
            settings.perAppVpnMode = if (checkedId == R.id.rbPerAppExclude) "EXCLUDE" else "INCLUDE"
            updateSelectAppsButtonText()
        }

        binding.btnSelectApps.setOnClickListener {
            showSelectAppsDialog()
        }

        updateSelectAppsButtonText()
    }

    private fun updateSelectAppsButtonText() {
        val count = settings.perAppVpnPackages.size
        binding.btnSelectApps.text = "📱 Выбрать приложения ($count выбрано)"
    }

    private fun showSelectAppsDialog() {
        val dialogView = LayoutInflater.from(requireContext()).inflate(R.layout.dialog_select_apps, null)
        val etSearch = dialogView.findViewById<EditText>(R.id.etSearchApp)
        val rvApps = dialogView.findViewById<RecyclerView>(R.id.rvAppsList)
        val btnClose = dialogView.findViewById<MaterialButton>(R.id.btnCloseAppsDialog)

        val pm = requireContext().packageManager
        val mainIntent = Intent(Intent.ACTION_MAIN, null).apply {
            addCategory(Intent.CATEGORY_LAUNCHER)
        }
        val resolveInfos = pm.queryIntentActivities(mainIntent, 0)
        val selectedSet = settings.perAppVpnPackages.toMutableSet()

        val appItems = resolveInfos.mapNotNull { ri ->
            val pkg = ri.activityInfo.packageName
            if (pkg == requireContext().packageName) return@mapNotNull null
            val name = ri.loadLabel(pm).toString()
            val icon = ri.loadIcon(pm)
            AppInfoItem(name = name, packageName = pkg, icon = icon, isSelected = selectedSet.contains(pkg))
        }.distinctBy { it.packageName }
         .sortedBy { it.name.lowercase() }

        val appsAdapter = AppSelectAdapter(appItems) { item ->
            settings.toggleAppPackage(item.packageName)
            updateSelectAppsButtonText()
        }

        rvApps.layoutManager = LinearLayoutManager(requireContext())
        rvApps.adapter = appsAdapter

        etSearch.doAfterTextChanged { text ->
            appsAdapter.filter(text?.toString() ?: "")
        }

        val dialog = MaterialAlertDialogBuilder(requireContext())
            .setView(dialogView)
            .create()

        btnClose.setOnClickListener {
            dialog.dismiss()
        }

        dialog.show()
    }

    override fun onResume() {
        super.onResume()
        refreshRulesList()
        updateSelectAppsButtonText()
    }

    override fun onDestroyView() {
        super.onDestroyView()
        _binding = null
    }
}
