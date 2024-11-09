import { defineConfig } from 'vitepress';

// https://vitepress.dev/reference/site-config
export default defineConfig({
    title: "Starina Documentation",
    themeConfig: {
        // https://vitepress.dev/reference/default-theme-config
        nav: [
            { text: 'Home', link: '/' },
            // { text: 'Examples', link: '/markdown-examples' }
        ],

        sidebar: [
            { text: 'Quickstart', link: '/quickstart' },
            // { text: 'Rust API Reference', link: '/rust', target: '_blank' },
            {
                text: 'Guides',
                items: [
                    { text: 'Writing Your First Application', link: '/guides/writing-your-first-application' },
                    { text: 'Writing Your First Server', link: '/guides/writing-your-first-server' },
                    { text: 'Writing Your First Device Driver', link: '/guides/writing-your-first-device-driver' },
                ]
            }
        ],

        socialLinks: [
            { icon: 'github', link: 'https://github.com/starina-os/starina' }
        ]
    }
})
