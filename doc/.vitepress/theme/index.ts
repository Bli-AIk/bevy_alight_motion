import DefaultTheme from 'vitepress/theme'
import './custom.css'
import AmPlayground from './components/AmPlayground.vue'
import ValidationReport from './components/ValidationReport.vue'
import FileUploader from './components/FileUploader.vue'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    // 注册全局组件，使其在 markdown 中可用
    app.component('AmPlayground', AmPlayground)
    app.component('ValidationReport', ValidationReport)
    app.component('FileUploader', FileUploader)
  }
}
