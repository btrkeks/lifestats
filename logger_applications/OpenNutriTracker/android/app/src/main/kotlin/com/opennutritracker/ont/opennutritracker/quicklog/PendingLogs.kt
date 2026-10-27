package com.opennutritracker.ont.opennutritracker.quicklog

import org.json.JSONObject

/** An uncertain reply belongs to the same user action on every retry. */
internal class PendingLogs(
    initial: Map<String, QuickLogCommand.Log> = emptyMap(),
    private val persist: (Map<String, QuickLogCommand.Log>) -> Unit,
) {
    private val pending = initial.toMutableMap()
    private val sending = mutableSetOf<String>()

    @Synchronized
    fun begin(key: String, create: () -> QuickLogCommand.Log): QuickLogCommand.Log? {
        if (!sending.add(key)) return null
        return pending.getOrPut(key, create).also { persist(pending.toMap()) }
    }

    @Synchronized
    fun finish(key: String, reply: QuickLogReply) {
        if (reply !is QuickLogReply.Unavailable) {
            pending.remove(key)
            persist(pending.toMap())
        }
        sending.remove(key)
    }

    @Synchronized
    fun inFlight(): Set<String> = sending.mapNotNull { pending[it]?.foodKey }.toSet()

    companion object {
        fun encode(commands: Map<String, QuickLogCommand.Log>): String =
            JSONObject(commands.mapValues { JSONObject(it.value.toWire()) }).toString()

        fun decode(json: String?): Map<String, QuickLogCommand.Log> = try {
            if (json == null) emptyMap() else JSONObject(json).let { root ->
                root.keys().asSequence().associateWith { key ->
                    val command = root.getJSONObject(key)
                    QuickLogCommand.Log(
                        command.getString("opId"), command.getString("foodKey"),
                        command.getString("profileId"), command.getDouble("baseAmount"),
                        command.getString("unit"), command.getLong("tappedAtMs"),
                    )
                }
            }
        } catch (_: Exception) {
            emptyMap()
        }
    }
}
