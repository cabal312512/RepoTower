import { pluginSummary } from '../plugins/registry';

export function openPluginPreview() {
  return { title: 'Plugins', items: pluginSummary() };
}
