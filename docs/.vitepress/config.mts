import { defineConfig } from 'vitepress';

// https://vitepress.dev/reference/site-config
export default defineConfig({
    title: "Starina Documentation",
    cleanUrls: true,

    // https://vitepress.dev/reference/default-theme-config
    themeConfig: {
        nav: [
            { text: 'Home', link: '/' },
        ],

        sidebar: [
            { text: 'Rust API Reference', link: '/rust/starina_api/', target: '_blank' },
            {
                text: 'Getting Started',
                items: [
                    { text: 'Quickstart', link: '/quickstart' },
                    { text: 'Your First Application', link: '/guides/writing-your-first-application' },
                ]
            },
            {
                text: 'Guides',
                items: [
                    { text: 'Your First Server', link: '/guides/writing-your-first-server' },
                    { text: 'Your First Device Driver', link: '/guides/writing-your-first-device-driver' },
                ]
            }
        ],

        socialLinks: [
            { icon: 'github', link: 'https://github.com/starina-os/starina' }
        ]
    }
})
