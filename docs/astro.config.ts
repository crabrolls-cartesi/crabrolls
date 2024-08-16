import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';
import starlightImageZoom from 'starlight-image-zoom';
import starlightViewModes from 'starlight-view-modes';

// https://astro.build/config
export default defineConfig({
	site: process.env.CI ? 'https://crabrolls-cartesi.github.io' : 'http://localhost:4321',
	base: '/crabrolls/',
	integrations: [
		starlight({
			title: 'CrabRolls',
			social: {
				github: 'https://github.com/crabrolls-cartesi/crabrolls',
			},
			editLink: {
				baseUrl: 'https://github.com/crabrolls-cartesi/crabrolls/tree/main/docs',
			},
			logo: {
				src: './src/assets/logo.png',
			},
			customCss: ['./src/styles/custom.css'],
			sidebar: [
				{
					label: 'Overview',
					autogenerate: {
						directory: 'overview',
					},
				},
				{
					label: 'Getting Started',
					autogenerate: {
						directory: 'getting-started',
					},
				},
				{
					label: 'Usage',
					autogenerate: {
						directory: 'usage',
					},
				},
			],
			pagination: true,
			plugins: [
				starlightLinksValidator({
					errorOnRelativeLinks: false,
				}),
				starlightImageZoom(),
				starlightViewModes(),
			],
		}),
	],
});
