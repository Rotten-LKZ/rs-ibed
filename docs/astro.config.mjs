// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightOpenApi, { openAPISidebarGroups } from 'starlight-openapi';
// https://astro.build/config
export default defineConfig({
	integrations: [
		starlight({
			plugins: [
				starlightOpenApi([
					{
						label: '[EN] API Reference',	
						base: 'en/api',
						schema: '../openapi.json',
					},
					{
						label: '[ZH] API Reference',
						base: 'zh/api',
						schema: '../openapi.json',
					}
				]),
			],
			title: 'RS-IBED',
			defaultLocale: 'en',
			locales: {
				zh: {
					label: '简体中文',
					lang: 'zh-CN',
				},
				en: {
					label: 'English',
					lang: 'en-US',
				},
			},
			social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/Rotten-LKZ/rs-ibed' }],
			sidebar: [
				{
					label: 'Guides',
					translations: {
						'zh-CN': '指南',
					},
					items: [
						{ label: 'Getting Started', slug: 'guides/getting-started', translations: { 'zh-CN': '开始使用' } },
						{ label: 'Config File', slug: 'guides/config', translations: { 'zh-CN': '配置文件' } },
						{ label: 'Development Guide', slug: 'guides/develop', translations: { 'zh-CN': '参与开发' } },
					],
				},
				...openAPISidebarGroups,
			],
		}),
	],
});
