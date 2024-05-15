import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';
import starlightLinksValidator from 'starlight-links-validator';
import starlightImageZoom from 'starlight-image-zoom';

// https://astro.build/config
export default defineConfig({
	site: process.env.CI ? 'http://vinicioslugli.github.io' : 'http://localhost:4321',
	base: '/crabrolls-cartesi/',
	integrations: [
		starlight({
			title: 'CrabRolls',
			social: {
				github: 'https://github.com/ViniciosLugli/crabrolls-cartesi',
			},
			editLink: {
				baseUrl: 'https://github.com/ViniciosLugli/crabrolls-cartesi/tree/main',
			},
			logo: {
				src: './src/assets/logo.png',
			},
			customCss: ['./src/styles/custom.css'],
			sidebar: [
				{
					label: 'Overview',
					autogenerate: { directory: 'overview' },
				},
			],
			pagination: true,
			plugins: [
				starlightLinksValidator({
					errorOnRelativeLinks: false,
				}),
				starlightImageZoom(),
			],
		}),
	],
});
