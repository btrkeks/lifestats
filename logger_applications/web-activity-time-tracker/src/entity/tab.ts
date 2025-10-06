import {todayLocalDate} from '../utils/date';
import {logger} from '../utils/logger';

export class Tab implements ISerializable<Tab> {
  static LOG_INTERVAL_SEC: number = 20;
  static secSinceLastLog: number = 0;

  url: string = '';
  favicon: string|undefined = '';
  summaryTime: number = 0;
  counter: number = 0;
  days: TabDay[] = [];

  init(url: string) { this.url = url; }

  private async sendLocalDbRequest(): Promise<void> {
    try {
      const response = await fetch('http://localhost:8000/api/log', {
        method : 'POST',
        headers : {
          'Content-Type' : 'application/json',
        },
	body: JSON.stringify({
	  statistic_id: 16,
	  update_policy: 'sum', 
	  value: `${Tab.LOG_INTERVAL_SEC}s`
	}),
      });

      if (!response.ok) {
	const errorMessage = await response.text();
	throw new Error(`HTTP error! status: ${response.status}, response: ${errorMessage}`);
      }
    } catch (error) {
      console.error('Error sending log:', error);
    }
  }

  private async logToLocalDB(): void {
    Tab.secSinceLastLog += 1;

    if (Tab.secSinceLastLog >= Tab.LOG_INTERVAL_SEC) {
      Tab.secSinceLastLog = 0;
      await this.sendLocalDbRequest();
    }
  }

  incSummaryTime(): void {
    this.summaryTime += 1;
    this.logToLocalDB();

    const day = this.days.find(x => x.date == todayLocalDate());
    if (day === undefined) {
      const newTab = this.addNewDay();
      newTab.incSummaryTime();
    } else
      day.incSummaryTime();
  }

  incCounter(): void {
    this.counter += 1;
    if (__DEV__)
      logger.log(`Counter ${this.url} - ${this.counter}`);
    const day = this.days.find(x => x.date == todayLocalDate());
    if (day === undefined) {
      const newTab = this.addNewDay();
      newTab.incCounter();
    } else
      day.incCounter();
  }

  addNewDay(): TabDay {
    const newTabDay = new TabDay();
    newTabDay.init(todayLocalDate());
    this.days.push(newTabDay);
    return newTabDay;
  }

  deserialize(input: Tab) {
    this.url = input.url;
    this.counter = input.counter;
    this.favicon = input.favicon;
    this.summaryTime = input.summaryTime;
    if (input.days?.length > 0)
      this.days = input.days.map(x => new TabDay().deserialize(x));

    return this;
  }

  setFavicon(favicon: string|undefined) { this.favicon = favicon; }
}

export class TabDay implements ISerializable<TabDay> {
  counter: number = 0;
  date: string = '';
  summary: number = 0;

  init(date: string) { this.date = date; }

  incSummaryTime(): void { this.summary += 1; }

  incCounter(): void { this.counter += 1; }

  deserialize(input: TabDay): TabDay {
    this.counter = input.counter;
    this.date = input.date;
    this.summary = input.summary;

    return this;
  }
}
