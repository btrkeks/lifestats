import { PhoneCalendar } from '@/models/calendar-sync';
import { unsupportedPhoneCalendar } from '@/services/unsupported-phone-calendar';

export function createPhoneCalendar(): PhoneCalendar {
  return unsupportedPhoneCalendar();
}
