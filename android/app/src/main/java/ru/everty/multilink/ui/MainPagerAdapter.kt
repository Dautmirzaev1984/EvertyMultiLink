package ru.everty.multilink.ui

import androidx.fragment.app.Fragment
import androidx.fragment.app.FragmentActivity
import androidx.viewpager2.adapter.FragmentStateAdapter

class MainPagerAdapter(
    activity: FragmentActivity,
    val homeFragment: HomeFragment = HomeFragment(),
    val serversFragment: ServersFragment = ServersFragment(),
    val routingFragment: RoutingFragment = RoutingFragment()
) : FragmentStateAdapter(activity) {

    override fun getItemCount(): Int = 3

    override fun createFragment(position: Int): Fragment {
        return when (position) {
            0 -> homeFragment
            1 -> serversFragment
            2 -> routingFragment
            else -> homeFragment
        }
    }
}
