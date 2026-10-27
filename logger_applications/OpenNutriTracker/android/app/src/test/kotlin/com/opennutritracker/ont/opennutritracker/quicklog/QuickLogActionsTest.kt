package com.opennutritracker.ont.opennutritracker.quicklog

import org.junit.Assert.*
import org.junit.Test
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicReference
import kotlin.concurrent.thread

class QuickLogActionsTest {
    private fun command(id: String, amount: Double = 250.0) =
        QuickLogCommand.Log(id, "name:quark", "a", amount, "g", 1793131200000L)

    @Test
    fun timeoutThenRetryAndUndoLeaveNoIntake() {
        var saved: Map<String, QuickLogCommand.Log> = emptyMap()
        val pending = PendingLogs { saved = it }
        val intakes = mutableSetOf<String>()
        val first = pending.begin("pill:a:quark") { command("first") }!!
        assertNull(pending.begin("pill:a:quark") { command("double-tap") })
        intakes.add(first.opId)
        pending.finish("pill:a:quark", QuickLogReply.Unavailable("timeout"))
        val restarted = PendingLogs(PendingLogs.decode(PendingLogs.encode(saved))) { saved = it }
        val retry = restarted.begin("pill:a:quark") { command("second", 180.0) }!!
        intakes.add(retry.opId)
        assertEquals(first, retry)
        assertEquals(1, intakes.size)
        restarted.finish("pill:a:quark", QuickLogReply.Ok("Added"))
        assertTrue(saved.isEmpty())
        intakes.remove(retry.opId)
        assertTrue(intakes.isEmpty())
        assertEquals("third", restarted.begin("pill:a:quark") { command("third") }!!.opId)
    }

    @Test
    fun definitiveRejectionAllowsAFreshSubmission() {
        val pending = PendingLogs { }
        pending.begin("sheet") { command("first") }
        pending.finish("sheet", QuickLogReply.Rejected("staleProfile"))
        assertEquals("new", pending.begin("sheet") { command("new") }!!.opId)
    }

    @Test
    fun rejectedOrUnavailableUndoNeverClaimsSuccess() {
        assertFalse(undoSucceeded(QuickLogReply.Rejected("staleProfile")))
        assertFalse(undoSucceeded(QuickLogReply.Unavailable("timeout")))
        assertTrue(undoSucceeded(QuickLogReply.Ok(null)))
    }

    @Test
    fun expiringTheOldActionCannotEraseTheNewUndoOnDisk() {
        val old = RecentAction("a", "name:quark", "a", "Added", 1000, 6000)
        val newer = old.copy(opId = "b")
        val disk = AtomicReference<RecentAction?>(old)
        val persistingNew = CountDownLatch(1)
        val allowPersist = CountDownLatch(1)
        val state = QuickLogState(WidgetModel(WidgetSnapshot.None, old, emptySet())) {
            disk.set(it)
            if (it?.opId == "b") {
                persistingNew.countDown()
                assertTrue(allowPersist.await(5, TimeUnit.SECONDS))
            }
        }
        val showing = thread { state.show(newer) }
        assertTrue(persistingNew.await(5, TimeUnit.SECONDS))
        val expired = CountDownLatch(1)
        val clearing = thread { state.clear("a"); expired.countDown() }
        expired.await(100, TimeUnit.MILLISECONDS)
        allowPersist.countDown()
        showing.join(5000)
        clearing.join(5000)
        assertFalse(showing.isAlive)
        assertFalse(clearing.isAlive)
        assertEquals(newer, state.flow.value.recentAction)
        assertEquals(newer, disk.get())
    }
}
