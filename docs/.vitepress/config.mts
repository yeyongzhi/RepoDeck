import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'zh-CN',
  title: 'RepoDeck',
  description: '本地 Git 仓库工作台',
  base: '/RepoDeck/',
  cleanUrls: true,
  themeConfig: {
    nav: [
      { text: '介绍', link: '/' },
      { text: '安装', link: '/install' },
      { text: '使用', link: '/usage' },
      { text: '功能', link: '/features' },
      { text: '更新日志', link: '/changelog' },
    ],
    sidebar: [
      { text: '产品文档', items: [
        { text: '软件介绍', link: '/' },
        { text: '安装说明', link: '/install' },
        { text: '使用说明', link: '/usage' },
        { text: '功能清单', link: '/features' },
        { text: '更新日志', link: '/changelog' },
      ] },
      { text: '开发资料', items: [
        { text: '项目说明书', link: '/project-guide' },
        { text: '技术设计', link: '/technical-design' },
        { text: '产品完善清单', link: '/product-improvements' },
        { text: '开发历史', link: '/development-history' },
      ] },
    ],
    socialLinks: [{ icon: 'github', link: 'https://github.com/yeyongzhi/RepoDeck' }],
    search: { provider: 'local' },
  },
})
