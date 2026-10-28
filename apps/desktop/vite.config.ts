import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
export default defineConfig({ plugins: [react()], clearScreen: false, base: './', server: { strictPort: true, proxy:{'/api/local-source':{target:'http://127.0.0.1:1421',changeOrigin:true}} } });
