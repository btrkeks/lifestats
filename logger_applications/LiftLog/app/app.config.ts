import { ConfigContext, ExpoConfig } from 'expo/config';

// A separate applicationId lets this fork install alongside the Play Store LiftLog without
// touching its data. Data moves across via Settings → Backup and restore.
export default ({ config }: ConfigContext): ExpoConfig => ({
  ...config,
  slug: config.slug ?? 'liftlog',
  name: 'LiftLog Dev',
  android: { ...config.android, package: 'com.limajuice.liftlog.dev' },
});
