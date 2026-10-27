package com.opennutritracker.ont.opennutritracker.quicklog

import androidx.work.ListenableWorker.Result
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Test

class QuickLogRefreshTest {
    @Test
    fun failedRefreshRetriesAndRecoveryCompletes() = runBlocking {
        for (reply in listOf(QuickLogReply.Unavailable("timeout"), QuickLogReply.Rejected("invalid"))) {
            assertEquals(Result.retry(), QuickLogTimerWorker.refresh { reply })
        }
        assertEquals(Result.success(), QuickLogTimerWorker.refresh { QuickLogReply.Ok(null) })
    }
}
