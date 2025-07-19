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
                    { text: 'Your First Server', link: '/tutorials/your-first-server' },
                    // { text: 'Your First Device Driver', link: '/tutorials/your-first-device-driver' },
                ],
            },
            {
                text: 'Concepts',
                items: [
                    { text: 'Channel', link: '/concepts/channel' },
                    { text: 'Poll', link: '/concepts/poll' },
                    { text: 'Startup', link: '/concepts/startup' },
                ],
            },
            {
                text: 'Linux Compatibility',
                items: [
                    { text: 'Running Linux containers', link: '/linux-compatibility/running-linux-containers.md' },
                ]
            },
            {
                text: 'Contributors Guide',
                items: [
                    { text: 'Kernel Development', link: '/contributors-guide/kernel-development.md' },
                    { text: 'Porting', link: '/contributors-guide/porting.md' },
                ]
            },
            {
                text: 'Apps',
                items: [
                    { text: 'TCP/IP server', link: '/apps/tcpip.md' },
                    { text: 'API server', link: '/apps/apiserver.md' },
                    { text: 'Virtio-net device driver', link: '/apps/virtio-net.md' },
                ]
            },
        ],

        socialLinks: [
            { icon: 'github', link: 'https://github.com/starina-os/starina' }
        ]
    }
})
