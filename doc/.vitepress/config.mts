import { defineConfig } from 'vitepress'

export default defineConfig({
  title: "bevy_alight_motion",
  description: "Alight Motion project parser and player for Bevy",
  
  head: [
    ['link', { rel: 'stylesheet', href: 'https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;700&family=Public+Sans:wght@400;700&display=swap' }]
  ],

  markdown: {
    lineNumbers: true
  },

  locales: {
    root: {
      label: 'English',
      lang: 'en',
      link: '/en/',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/en/guide/introduction' },
          { text: 'Effects', link: '/en/effects/' },
          { text: 'Examples', link: '/en/examples/' },
          { text: 'Playground', link: '/en/playground/' }
        ],
        sidebar: {
          '/en/guide/': [
            {
              text: 'Getting Started',
              items: [
                { text: 'Introduction', link: '/en/guide/introduction' },
                { text: 'Installation', link: '/en/guide/installation' },
                { text: 'Quick Start', link: '/en/guide/quick-start' }
              ]
            },
            {
              text: 'Architecture',
              items: [
                { text: 'Overview', link: '/en/guide/architecture' },
                { text: 'SDF Rendering', link: '/en/guide/sdf' },
                { text: 'RTT & Effects', link: '/en/guide/rtt' }
              ]
            }
          ],
          '/en/effects/': [
            {
              text: 'Basic Features',
              items: [
                { text: 'Transform & Movement', link: '/en/effects/transform' },
                { text: 'Shapes & Fills', link: '/en/effects/shapes' },
                { text: 'Groups & Resolution', link: '/en/effects/groups' },
                { text: 'Easings', link: '/en/effects/easings' }
              ]
            },
            {
              text: 'Advanced Effects',
              items: [
                { text: 'Wipe', link: '/en/effects/wipe' },
                { text: 'Gaussian Blur', link: '/en/effects/gaussian-blur' },
                { text: 'Stretch Segment', link: '/en/effects/stretch-segment' },
                { text: 'Palette Map', link: '/en/effects/palette-map' },
                { text: 'Replace Color', link: '/en/effects/replace-color' },
                { text: 'Scale Assist', link: '/en/effects/scale-assist' }
              ]
            },
            {
              text: 'Masking',
              items: [
                { text: 'Layer Masks', link: '/en/effects/masking' }
              ]
            }
          ]
        }
      }
    },
    'zh-hans': {
      label: '简体中文',
      lang: 'zh-Hans',
      link: '/zh-hans/',
      themeConfig: {
        nav: [
          { text: '指南', link: '/zh-hans/guide/introduction' },
          { text: '效果', link: '/zh-hans/effects/' },
          { text: '示例', link: '/zh-hans/examples/' },
          { text: 'Playground', link: '/zh-hans/playground/' }
        ],
        sidebar: {
          '/zh-hans/guide/': [
            {
              text: '入门指南',
              items: [
                { text: '项目介绍', link: '/zh-hans/guide/introduction' },
                { text: '安装', link: '/zh-hans/guide/installation' },
                { text: '快速开始', link: '/zh-hans/guide/quick-start' }
              ]
            },
            {
              text: '核心架构',
              items: [
                { text: '架构总览', link: '/zh-hans/guide/architecture' },
                { text: 'SDF 渲染', link: '/zh-hans/guide/sdf' },
                { text: 'RTT 与效果系统', link: '/zh-hans/guide/rtt' }
              ]
            }
          ],
          '/zh-hans/effects/': [
            {
              text: '基础功能',
              items: [
                { text: '变换与移动', link: '/zh-hans/effects/transform' },
                { text: '形状与填充', link: '/zh-hans/effects/shapes' },
                { text: '编组与分辨率', link: '/zh-hans/effects/groups' },
                { text: '缓动曲线', link: '/zh-hans/effects/easings' }
              ]
            },
            {
              text: '高级效果',
              items: [
                { text: '擦拭 (Wipe)', link: '/zh-hans/effects/wipe' },
                { text: '高斯模糊 (Gaussian Blur)', link: '/zh-hans/effects/gaussian-blur' },
                { text: '拉伸片段 (Stretch Segment)', link: '/zh-hans/effects/stretch-segment' },
                { text: '调色板映射 (Palette Map)', link: '/zh-hans/effects/palette-map' },
                { text: '颜色替换 (Replace Color)', link: '/zh-hans/effects/replace-color' },
                { text: '缩放辅助 (Scale Assist)', link: '/zh-hans/effects/scale-assist' }
              ]
            },
            {
              text: '遮罩',
              items: [
                { text: '图层遮罩', link: '/zh-hans/effects/masking' }
              ]
            }
          ]
        }
      }
    }
  },

  themeConfig: {
    lastUpdated: {
      text: 'Last Forged',
      formatOptions: {
        dateStyle: 'medium',
        timeStyle: 'short'
      }
    },
    search: {
      provider: 'local',
      options: {
        locales: {
          'zh-hans': {
            translations: {
              button: { buttonText: '搜索文档', buttonAriaLabel: '搜索文档' },
              modal: { noResultsText: '未找到相关指令', footer: { selectText: '选择', navigateText: '切换' } }
            }
          }
        }
      }
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/Bli-AIk/bevy_alight_motion' }
    ],
    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2024-present'
    }
  }
})
