ALTER TABLE `calendar_claim` ADD `title` text DEFAULT '' NOT NULL;--> statement-breakpoint
ALTER TABLE `calendar_claim` ADD `organizer` text;--> statement-breakpoint
ALTER TABLE `calendar_claim` ADD `sourceStartMs` integer;--> statement-breakpoint
ALTER TABLE `calendar_claim` ADD `sourceDuration` text;--> statement-breakpoint
ALTER TABLE `calendar_claim` ADD `sourceTimezone` text;