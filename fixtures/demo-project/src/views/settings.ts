import { accountCard } from '../features/account';

export function settingsView() {
  return { title: 'Settings', account: accountCard() };
}
