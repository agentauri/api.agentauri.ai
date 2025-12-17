// @ts-check
import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightOpenAPI, { openAPISidebarGroups } from 'starlight-openapi';

// https://astro.build/config
export default defineConfig({
	site: 'https://docs.agentauri.ai',
	integrations: [
		starlight({
			title: 'AgentAuri API',
			logo: {
				src: './src/assets/logo.svg',
			},
			social: [
				{ icon: 'github', label: 'GitHub', href: 'https://github.com/agentauri/api.agentauri.ai' },
			],
			customCss: ['./src/styles/custom.css'],
			plugins: [
				starlightOpenAPI([
					{
						base: 'api',
						label: 'API Reference',
						schema: './src/schemas/openapi.json',
					},
				]),
			],
			sidebar: [
				{
					label: 'Getting Started',
					items: [
						{ label: 'Introduction', slug: 'index' },
						{ label: 'Quickstart', slug: 'getting-started/quickstart' },
						{ label: 'Authentication', slug: 'getting-started/authentication' },
						{ label: 'API Keys', slug: 'getting-started/api-keys' },
					],
				},
				{
					label: 'Concepts',
					items: [
						{ label: 'Triggers', slug: 'concepts/triggers' },
						{ label: 'Actions', slug: 'concepts/actions' },
						{ label: 'Events', slug: 'concepts/events' },
					],
				},
				{
					label: 'Guides',
					items: [
						{ label: 'Webhook Integration', slug: 'guides/webhook-integration' },
						{ label: 'Telegram Notifications', slug: 'guides/telegram-notifications' },
					],
				},
				...openAPISidebarGroups,
			],
		}),
	],
});
