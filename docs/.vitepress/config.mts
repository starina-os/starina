import { defineConfig } from 'vitepress'

// https://vitepress.dev/reference/site-config
export default defineConfig({
    title: "Starina Documentation",
    description: "Learn Starina",
    themeConfig: {
        nav: [
            { text: 'Home', link: '/' },
            { text: 'Getting Started', link: '/getting-started' },
        ],

        sidebar: [
            { text: 'Getting Started', link: '/getting-started' },
            { text: 'Comparison with Others', link: '/comparison-with-others' },
            {
                text: 'Contributors Guide', items: [
                    { text: 'Kernel Development', link: '/contributors-guide/kernel-development.md' },
                    { text: 'Porting', link: '/contributors-guide/porting.md' },
                ]
            },
        ],

        socialLinks: [
            { icon: 'github', link: 'https://github.com/vuejs/vitepress' }
        ]
    }
})
