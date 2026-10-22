CREATE TABLE `calendar_claim` (
	`sessionId` text PRIMARY KEY NOT NULL,
	`calendarId` integer NOT NULL,
	`accountName` text NOT NULL,
	`eventId` integer NOT NULL,
	`instanceTime` integer NOT NULL,
	`syncId` text,
	`rrule` text,
	`date` text NOT NULL,
	`originalStartMs` integer NOT NULL,
	`originalEndMs` integer NOT NULL,
	`observedStartMs` integer NOT NULL,
	`observedEndMs` integer NOT NULL,
	`attemptStartMs` integer,
	`attemptEndMs` integer,
	FOREIGN KEY (`sessionId`) REFERENCES `calendar_sync_session`(`sessionId`) ON UPDATE no action ON DELETE no action
);
--> statement-breakpoint
CREATE UNIQUE INDEX `calendar_claim_occurrence` ON `calendar_claim` (`calendarId`,`eventId`,`instanceTime`);--> statement-breakpoint
CREATE TABLE `calendar_sync_session` (
	`sessionId` text PRIMARY KEY NOT NULL,
	`seq` integer DEFAULT 1 NOT NULL,
	`doneSeq` integer DEFAULT 0 NOT NULL,
	`calendarId` integer NOT NULL,
	`accountName` text NOT NULL,
	`title` text NOT NULL,
	`outcome` text,
	`outcomeDate` text
);
