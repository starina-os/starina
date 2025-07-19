import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
    title: "Starina Documentation",
    description: "Learn Starina",
    cleanUrls: true,
    themeConfig: {
        nav: [
            { text: 'Home', link: '/' },
            { text: 'Getting Started', link: '/getting-started' },
        ],

        sidebar: [
            { text: 'Getting Started', link: '/getting-started' },
            { text: 'Comparison with Others', link: '/comparison-with-others' },
            {
                text: 'Tutorials',
                items: [
                    { text: 'Your First App', link: '/tutorials/your-first-app' },
                ],
            },
            {
                text: 'Concepts',
                items: [
                    { text: 'Channel', link: '/concepts/channel' },
                ],
            },
            {
                text: 'Contributors Guide',
                items: [
                    { text: 'Kernel Development', link: '/contributors-guide/kernel-development.md' },
                    { text: 'Porting', link: '/contributors-guide/porting.md' },
                ]
            },
        ],

        socialLinks: [
            { icon: 'github', link: 'https://github.com/starina-os/starina' }
        ]
    }
})
