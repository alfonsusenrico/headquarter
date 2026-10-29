package com.alfonsusenrico.headquarter.ui

/**
 * [items] with every one whose key was in [shown] back in the order it had there; an item new to
 * the list keeps its place. Rows hold still while the owner touches the list, as on the desktop
 * while the pointer is on the sidebar (add-working-sessions-first D3).
 */
fun <T> heldOrder(
    items: List<T>,
    shown: List<String>,
    key: (T) -> String,
): List<T> {
    val rank = shown.withIndex().associate { (place, id) -> id to place }
    val slots = items.indices.filter { rank.containsKey(key(items[it])) }
    val known = slots.map { items[it] }.sortedBy { rank.getValue(key(it)) }
    val held = items.toMutableList()
    slots.zip(known).forEach { (slot, item) -> held[slot] = item }
    return held
}
