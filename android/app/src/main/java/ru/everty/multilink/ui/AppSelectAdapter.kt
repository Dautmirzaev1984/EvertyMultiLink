package ru.everty.multilink.ui

import android.graphics.drawable.Drawable
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.CheckBox
import android.widget.ImageView
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import ru.everty.multilink.R

data class AppInfoItem(
    val name: String,
    val packageName: String,
    val icon: Drawable?,
    var isSelected: Boolean
)

class AppSelectAdapter(
    private var allApps: List<AppInfoItem>,
    private val onToggle: (AppInfoItem) -> Unit
) : RecyclerView.Adapter<AppSelectAdapter.AppViewHolder>() {

    private var filteredApps: List<AppInfoItem> = allApps

    fun setApps(apps: List<AppInfoItem>) {
        allApps = apps
        filteredApps = apps
        notifyDataSetChanged()
    }

    fun filter(query: String) {
        filteredApps = if (query.isBlank()) {
            allApps
        } else {
            allApps.filter {
                it.name.contains(query, ignoreCase = true) || it.packageName.contains(query, ignoreCase = true)
            }
        }
        notifyDataSetChanged()
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): AppViewHolder {
        val view = LayoutInflater.from(parent.context).inflate(R.layout.item_app_select, parent, false)
        return AppViewHolder(view)
    }

    override fun onBindViewHolder(holder: AppViewHolder, position: Int) {
        holder.bind(filteredApps[position])
    }

    override fun getItemCount(): Int = filteredApps.size

    inner class AppViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        private val ivIcon: ImageView = itemView.findViewById(R.id.ivAppIcon)
        private val tvName: TextView = itemView.findViewById(R.id.tvAppName)
        private val tvPkg: TextView = itemView.findViewById(R.id.tvAppPackage)
        private val cbSelected: CheckBox = itemView.findViewById(R.id.cbAppSelected)

        init {
            itemView.setOnClickListener {
                val pos = bindingAdapterPosition
                if (pos != RecyclerView.NO_POSITION) {
                    val item = filteredApps[pos]
                    item.isSelected = !item.isSelected
                    cbSelected.isChecked = item.isSelected
                    onToggle(item)
                }
            }
        }

        fun bind(item: AppInfoItem) {
            tvName.text = item.name
            tvPkg.text = item.packageName
            cbSelected.isChecked = item.isSelected
            if (item.icon != null) {
                ivIcon.setImageDrawable(item.icon)
            } else {
                ivIcon.setImageResource(R.mipmap.ic_launcher)
            }
        }
    }
}
