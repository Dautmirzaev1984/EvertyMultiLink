package ru.everty.multilink.service

object Tun2Socks {
    init {
        try {
            System.loadLibrary("tun2socks")
        } catch (e: Throwable) {
            e.printStackTrace()
        }
    }

    @JvmStatic
    external fun start(fd: Int, proxy: String, mtu: Int): Int

    @JvmStatic
    external fun stop(): Int
}
