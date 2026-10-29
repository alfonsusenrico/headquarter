package com.alfonsusenrico.headquarter.ui

import uniffi.hq_mobile.MobileException

/**
 * The sentence to show the owner for an error from the phone's core. UniFFI builds `message` from
 * the fields (`reason=…`) and leaves it empty without one, so screens use this instead
 * (fix-phone-error-text D1).
 */
fun MobileException.ownerText(): String =
    when (this) {
        is MobileException.Failed -> reason
        is MobileException.Refused -> reason
        is MobileException.NotConnected -> "Not connected to the computer."
    }
