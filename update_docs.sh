#!/bin/bash
# update_docs.sh - 自动化测试和文档生成脚本
# Automated testing and documentation generation script

set -e

# 颜色定义 / Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 函数：显示菜单 / Function: Display menu
show_menu() {
    echo ""
    echo -e "${CYAN}============================================${NC}"
    echo -e "${CYAN}   bevy_alight_motion 文档更新工具${NC}"
    echo -e "${CYAN}   Documentation Update Tool${NC}"
    echo -e "${CYAN}============================================${NC}"
    echo ""
    echo -e "${YELLOW}请选择操作 / Please select an option:${NC}"
    echo ""
    echo -e "  ${GREEN}1)${NC} 更新全部测试并生成文档 ${YELLOW}(默认 / default)${NC}"
    echo -e "     ${BLUE}Run all tests and generate documentation${NC}"
    echo ""
    echo -e "  ${GREEN}2)${NC} 只更新全部测试"
    echo -e "     ${BLUE}Run all tests only${NC}"
    echo ""
    echo -e "  ${GREEN}3)${NC} 只生成文档"
    echo -e "     ${BLUE}Generate documentation only${NC}"
    echo ""
    echo -e "  ${GREEN}4)${NC} 更新部分测试并生成文档"
    echo -e "     ${BLUE}Run specific tests and generate documentation${NC}"
    echo ""
    echo -e "  ${GREEN}5)${NC} 只更新部分测试"
    echo -e "     ${BLUE}Run specific tests only${NC}"
    echo ""
    echo -e "  ${GREEN}q)${NC} 退出 / Exit"
    echo ""
}

# 函数：运行全部测试 / Function: Run all tests
run_all_tests() {
    echo ""
    echo -e "${YELLOW}▶ 开始运行全部测试... / Running all tests...${NC}"
    echo ""
    ./test_comparison.sh --all
    echo ""
    echo -e "${GREEN}✅ 全部测试完成 / All tests completed${NC}"
}

# 函数：运行部分测试 / Function: Run specific tests
run_specific_tests() {
    local tests=$1
    echo ""
    echo -e "${YELLOW}▶ 开始运行指定测试: $tests${NC}"
    echo -e "${YELLOW}  Running specific tests: $tests${NC}"
    echo ""
    ./test_comparison.sh $tests
    echo ""
    echo -e "${GREEN}✅ 指定测试完成 / Specific tests completed${NC}"
}

# 函数：生成文档 / Function: Generate documentation
generate_docs() {
    echo ""
    echo -e "${YELLOW}▶ 开始生成文档... / Generating documentation...${NC}"
    echo ""
    cargo run --example generate_docs
    echo ""
    echo -e "${GREEN}✅ 文档生成完成 / Documentation generated${NC}"
    echo ""
    echo -e "${CYAN}生成的文件 / Generated files:${NC}"
    echo "  doc/zh-hans/effects/"
    echo "  doc/zh-hans/builtins/"
    echo "  doc/en/effects/"
    echo "  doc/en/builtins/"
    echo "  doc/.vitepress/sidebar-effects.mts"
}

# 函数：获取用户输入的测试名称 / Function: Get test names from user
# 使用全局变量 TEST_NAMES 返回结果
prompt_for_test_names() {
    TEST_NAMES=""
    echo ""
    echo -e "${CYAN}════════════════════════════════════════════${NC}"
    echo -e "${CYAN}  输入要运行的测试名称${NC}"
    echo -e "${CYAN}  Enter test names to run${NC}"
    echo -e "${CYAN}════════════════════════════════════════════${NC}"
    echo ""
    echo -e "${YELLOW}可用的测试文件 / Available test files:${NC}"
    echo ""
    # 列出可用测试
    if [ -f "comparison_config.toml" ]; then
        grep -E '^\[projects\.' comparison_config.toml | sed 's/\[projects\.\(.*\)\]/  \1/' | head -30
        echo ""
        echo -e "${BLUE}提示：多个测试用空格分隔${NC}"
        echo -e "${BLUE}Tip: Separate multiple tests with spaces${NC}"
        echo -e "${BLUE}例如 / Example: basic_shape fx_1_stretch_segment${NC}"
    fi
    echo ""
    echo -n -e "${GREEN}请输入测试名称 / Enter test names: ${NC}"
    read -r TEST_NAMES
    if [ -z "$TEST_NAMES" ]; then
        echo -e "${RED}❌ 未输入测试名称，操作取消 / No test names entered, operation cancelled${NC}"
        return 1
    fi
    return 0
}

# 主循环 / Main loop
main() {
    cd "$(dirname "$0")"
    
    while true; do
        show_menu
        echo -n -e "${GREEN}请输入选项 / Enter option: ${NC}"
        read -r choice
        
        case $choice in
            1)
                run_all_tests
                generate_docs
                echo ""
                echo -e "${GREEN}════════════════════════════════════════${NC}"
                echo -e "${GREEN}✅ 全部完成！/ All done!${NC}"
                echo -e "${GREEN}════════════════════════════════════════${NC}"
                ;;
            2)
                run_all_tests
                ;;
            3)
                generate_docs
                ;;
            4)
                prompt_for_test_names
                if [ $? -eq 0 ] && [ -n "$TEST_NAMES" ]; then
                    run_specific_tests "$TEST_NAMES"
                    generate_docs
                    echo ""
                    echo -e "${GREEN}════════════════════════════════════════${NC}"
                    echo -e "${GREEN}✅ 全部完成！/ All done!${NC}"
                    echo -e "${GREEN}════════════════════════════════════════${NC}"
                fi
                ;;
            5)
                prompt_for_test_names
                if [ $? -eq 0 ] && [ -n "$TEST_NAMES" ]; then
                    run_specific_tests "$TEST_NAMES"
                fi
                ;;
            q|Q)
                echo ""
                echo -e "${CYAN}再见！/ Goodbye!${NC}"
                exit 0
                ;;
            *)
                # 默认执行选项 1 / Default to option 1
                echo ""
                echo -e "${YELLOW}使用默认选项：更新全部测试并生成文档${NC}"
                echo -e "${YELLOW}Using default: Run all tests and generate documentation${NC}"
                run_all_tests
                generate_docs
                echo ""
                echo -e "${GREEN}════════════════════════════════════════${NC}"
                echo -e "${GREEN}✅ 全部完成！/ All done!${NC}"
                echo -e "${GREEN}════════════════════════════════════════${NC}"
                ;;
        esac
        
        echo ""
        echo -e "${YELLOW}按 Enter 继续... / Press Enter to continue...${NC}"
        read -r
    done
}

# 运行主函数 / Run main function
main
