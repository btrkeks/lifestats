import { KeyValueStore } from '@/services/key-value-store';
import { PrefDescriptor, preferenceRegistry, PrefKey, PrefValue } from '@/store/settings/registry';

export class PreferenceService {
  constructor(private keyValueStore: KeyValueStore) {}

  async getPreference<K extends PrefKey>(key: K): Promise<PrefValue<K>> {
    const descriptor = preferenceRegistry[key] as PrefDescriptor<PrefValue<K>>;
    const storageKey = descriptor.storageKey ?? key;
    if (!descriptor.codec) {
      return descriptor.default;
    }
    const raw = descriptor.sync
      ? this.keyValueStore.getItemSync(storageKey)
      : await this.keyValueStore.getItem(storageKey);
    return descriptor.codec.deserialize(raw) ?? descriptor.default;
  }

  async setPreference<K extends PrefKey>(key: K, value: PrefValue<K>): Promise<void> {
    const descriptor = preferenceRegistry[key] as PrefDescriptor<PrefValue<K>>;
    if (!descriptor.codec) {
      return;
    }
    const storageKey = descriptor.storageKey ?? key;
    const serialized = descriptor.codec.serialize(value);
    if (serialized === undefined) {
      await this.keyValueStore.removeItem(storageKey);
    } else {
      await this.keyValueStore.setItem(storageKey, serialized);
    }
  }

  getUseImperialUnits(): Promise<boolean> {
    return this.getPreference('useImperialUnits');
  }

  // Sync so it can be read before the store exists (Tolgee bootstrap). Rewrites
  // the legacy `zh_Hans` value to the current `zh-hans` code on read.
  getPreferredLanguage(): string | undefined {
    const lang = this.keyValueStore.getItemSync('preferredLanguage');
    if (lang === 'zh_Hans') {
      void this.keyValueStore.setItem('preferredLanguage', 'zh-hans');
      return 'zh-hans';
    }
    return lang;
  }

  setPreferredLanguage(lang: string | undefined): Promise<void> {
    return lang
      ? this.keyValueStore.setItem('preferredLanguage', lang)
      : this.keyValueStore.removeItem('preferredLanguage');
  }
}
