import { defineConfig } from '@playwright/test';

export default defineConfig({
	workers: 1,
	use: {
		baseURL: 'http://127.0.0.1:4173',
		trace: 'on-first-retry'
	},
	webServer: [
		{
			command: 'node e2e/mock-backend.mjs',
			url: 'http://127.0.0.1:3100/__health'
		},
		{
			command:
				'BACKEND_ORIGIN=http://127.0.0.1:3100 npm run build && BACKEND_ORIGIN=http://127.0.0.1:3100 npm run preview -- --host 127.0.0.1 --port 4173',
			url: 'http://127.0.0.1:4173'
		}
	],
	testMatch: '**/*.e2e.{ts,js}'
});
