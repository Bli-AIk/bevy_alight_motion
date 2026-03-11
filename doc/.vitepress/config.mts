import { defineConfig } from 'vitepress'
import { zhHansEffects, enEffects, zhHansBuiltins, enBuiltins } from './sidebar-effects.mts'

export default defineConfig({
  title: "bevy_alight_motion",
  description: "Alight Motion project parser and player for Bevy",
  base: '/bevy_alight_motion/',
  
  head: [
    ['link', { rel: 'stylesheet', href: 'https://fonts.googleapis.com/css2?family=JetBrains+Mono:wght@400;700&family=Public+Sans:wght@400;700&display=swap' }],
    ['meta', { name: 'viewport', content: 'width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no, viewport-fit=cover' }]
  ],

  markdown: {
    lineNumbers: true
  },

  locales: {
    en: {
      label: 'English',
      lang: 'en',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/en/guide/introduction' },
          { text: 'Graphics', link: '/en/builtins/' },
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
              text: 'Concepts',
              items: [
                { text: 'Transform & Movement', link: '/en/effects/transform' },
                { text: 'Shapes & Fills', link: '/en/effects/shapes' },
                { text: 'Groups & Resolution', link: '/en/effects/groups' },
                { text: 'Easings', link: '/en/effects/easings' }
              ]
            },
            enEffects,
            {
              text: 'Masking',
              items: [
                { text: 'Layer Masks', link: '/en/effects/masking' }
              ]
            }
          ],
          '/en/builtins/': enBuiltins
        }
      }
    },
    'zh-hans': {
      label: '简体中文',
      lang: 'zh-Hans',
      themeConfig: {
        nav: [
          { text: '指南', link: '/zh-hans/guide/introduction' },
          { text: '图形元素', link: '/zh-hans/builtins/' },
          { text: '效果', link: '/zh-hans/effects/' },
          { text: '示例', link: '/zh-hans/examples/' },
          { text: '试玩场', link: '/zh-hans/playground/' }
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
              text: '概念总览',
              items: [
                { text: '变换与移动', link: '/zh-hans/effects/transform' },
                { text: '形状与填充', link: '/zh-hans/effects/shapes' },
                { text: '编组与分辨率', link: '/zh-hans/effects/groups' },
                { text: '缓动曲线', link: '/zh-hans/effects/easings' }
              ]
            },
            zhHansEffects,
            {
              text: '遮罩',
              items: [
                { text: '图层遮罩', link: '/zh-hans/effects/masking' }
              ]
            }
          ],
          '/zh-hans/builtins/': zhHansBuiltins
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
