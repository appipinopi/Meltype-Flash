// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// 任意導入したshuntiを有界なUTF-8行プロトコルで呼ぶWindows用ヘルパー。
#include "engine.h"
#include <algorithm>
#include <codecvt>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <locale>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>
#define WIN32_LEAN_AND_MEAN
#include <windows.h>

static std::vector<char> read(const std::filesystem::path& path) {
    std::ifstream stream(path, std::ios::binary | std::ios::ate);
    const auto size = stream.tellg();
    if (!stream || size <= 0 || size > 512 * 1024 * 1024)
        throw std::runtime_error("Invalid shunti model/dictionary file");
    std::vector<char> data(static_cast<size_t>(size));
    stream.seekg(0);
    if (!stream.read(data.data(), size)) throw std::runtime_error("Cannot read shunti data");
    return data;
}

int main() {
    try {
        std::vector<wchar_t> filename(32768);
        const auto n = GetModuleFileNameW(nullptr, filename.data(), static_cast<DWORD>(filename.size()));
        if (!n || n == filename.size()) throw std::runtime_error("Cannot locate shunti executable");
        const auto root = std::filesystem::path(std::wstring(filename.data(), n)).parent_path();
        // Wide file paths support Japanese Windows user names. Buffers outlive the engine.
        const auto lex = read(root / L"kkc_lex.bin");
        const auto model = read(root / L"kkc_model.bin");
        std::unique_ptr<kkc_engine, decltype(&kkc_close)> engine(
            kkc_open(lex.data(), lex.size(), model.data(), model.size()), kkc_close);
        if (!engine) throw std::runtime_error("Cannot open shunti model/dictionary");
        kkc_set_threads(engine.get(), 2);
        std::wstring_convert<std::codecvt_utf8_utf16<char16_t>, char16_t> utf;
        std::vector<char> line(65538);
        std::vector<uint16_t> output(65536);
        std::cout << "READY" << std::endl;
        while (std::cin.getline(line.data(), static_cast<std::streamsize>(line.size()))) {
            std::string input(line.data());
            if (!input.empty() && input.back() == '\r') input.pop_back();
            if (input.size() > 65536) break;
            const auto tab = input.find('\t', 2);
            if (input.rfind("C\t", 0) != 0 || tab == std::string::npos || input.find('\t', tab + 1) != std::string::npos) {
                std::cout << "ERR" << std::endl;
                continue;
            }
            try {
                auto context = utf.from_bytes(input.substr(2, tab - 2));
                const auto reading = input.substr(tab + 1);
                const auto kana = utf.from_bytes(reading);
                if (kana.empty() || kana.size() > 1024) {
                    std::cout << std::endl;
                    continue;
                }
                if (context.size() > 160) {
                    auto start = context.size() - 160;
                    if (context[start] >= 0xdc00 && context[start] <= 0xdfff) ++start;
                    context.erase(0, start);
                }
                std::fill(output.begin(), output.end(), 0);
                const auto count = kkc_convert(engine.get(),
                    reinterpret_cast<const uint16_t*>(context.data()), static_cast<int>(context.size()),
                    reinterpret_cast<const uint16_t*>(kana.data()), static_cast<int>(kana.size()),
                    8, 1, output.data(), static_cast<int>(output.size()));
                std::string response;
                size_t offset = 0;
                for (int i = 0; i < count && i < 8; ++i) {
                    const auto end = std::find(output.begin() + offset, output.end(), 0);
                    if (end == output.end()) throw std::runtime_error("Invalid shunti output");
                    auto text = utf.to_bytes(reinterpret_cast<const char16_t*>(output.data() + offset),
                        reinterpret_cast<const char16_t*>(output.data() + (end - output.begin())));
                    for (auto& c : text) if (c == '\x1e' || c == '\x1f' || c == '\t' || c == '\r' || c == '\n') c = ' ';
                    if (!text.empty()) response += '\x1f' + text;
                    offset = static_cast<size_t>(end - output.begin()) + 1;
                    if (offset >= output.size()) break;
                }
                std::cout << (response.empty() ? "" : reading + response) << std::endl;
            } catch (const std::exception&) {
                std::cout << "ERR" << std::endl;
            }
        }
        return 0;
    } catch (const std::exception& error) {
        std::cerr << error.what() << std::endl;
        return 1;
    }
}
