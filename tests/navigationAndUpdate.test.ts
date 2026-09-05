import { describe, expect, test } from 'bun:test';
import { canOpenAppPage, isAlwaysAvailablePage } from '../src/navigation';
import { oauthSubpages } from '../src/oauthNavigation';

describe('首页与设置导航', () => {
  test('内核停止时只有首页和设置可进入', () => {
    expect(isAlwaysAvailablePage('home')).toBe(true);
    expect(isAlwaysAvailablePage('config')).toBe(true);
    expect(canOpenAppPage('home', false)).toBe(true);
    expect(canOpenAppPage('config', false)).toBe(true);
    expect(canOpenAppPage('oauth', false)).toBe(false);
    expect(canOpenAppPage('agents', false)).toBe(false);
  });

  test('被移除的页面不再是常驻页', () => {
    for (const removed of ['easy', 'versions', 'usage-records', 'api']) {
      expect(isAlwaysAvailablePage(removed)).toBe(false);
      expect(canOpenAppPage(removed, false)).toBe(false);
    }
  });

  test('内核运行后解锁其他功能页', () => {
    expect(canOpenAppPage('oauth', true)).toBe(true);
    expect(canOpenAppPage('agents', true)).toBe(true);
  });
});

describe('OAuth 子页面导航', () => {
  test('认证文件和额度查询收纳在 OAuth 页面内', () => {
    expect(oauthSubpages.map((page) => page.id)).toEqual(['login', 'authFiles', 'quota']);
    expect(oauthSubpages.map((page) => page.labelKey)).toEqual([
      'oauth.title',
      'authFiles.title',
      'quota.title',
    ]);
  });
});
