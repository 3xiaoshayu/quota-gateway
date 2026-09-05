import { describe, expect, it } from 'bun:test';
import { getCurrentLocale, normalizeLocale, translate } from '../src/i18n';
import { zhCN } from '../src/i18n/resources';

describe('i18n', () => {
  it('is Simplified Chinese only; every locale hint normalizes to zh-CN', () => {
    expect(getCurrentLocale()).toBe('zh-CN');
    for (const hint of ['en-US', 'ja-JP', 'zh-TW', 'zh-Hant-HK', 'unsupported', null, undefined]) {
      expect(normalizeLocale(hint)).toBe('zh-CN');
    }
  });

  it('translates messages and interpolates variables', () => {
    expect(translate('zh-CN', 'kernel.install.installingVersion', { version: '1.2.3' }))
      .toBe('正在安装 1.2.3');
    expect(translate('zh-CN', 'kernel.access.openaiDescription')).toBe('OpenAI兼容格式');
  });

  it('carries the product name and no leftover upstream brand', () => {
    expect(zhCN['app.productName']).toBe('Quota Gateway');
    const leaked = Object.entries(zhCN).filter(([, value]) => /EasyCLIProxyAPI/.test(value));
    expect(leaked).toEqual([]);
  });
});
