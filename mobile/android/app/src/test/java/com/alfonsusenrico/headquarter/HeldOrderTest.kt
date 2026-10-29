package com.alfonsusenrico.headquarter

import com.alfonsusenrico.headquarter.ui.heldOrder
import org.junit.Assert.assertEquals
import org.junit.Test

class HeldOrderTest {
    @Test
    fun heldRowsKeepTheirPlacesAndNewOnesTakeTheirs() {
        // Shown: finished, then cash. Now cash works and comes first, and a new row appears.
        val computed = listOf("cash", "new", "finished")
        val held = heldOrder(computed, listOf("finished", "cash")) { it }
        assertEquals(listOf("finished", "new", "cash"), held)
        assertEquals(computed, heldOrder(computed, emptyList()) { it })
    }
}
