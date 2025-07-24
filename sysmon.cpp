#include <iostream>
#include <cstdlib>
#include <string>
#include <memory>
#include <array>
#include <regex>
#include <sys/types.h>
#include <sys/sysctl.h>
#include <algorithm>
#include <sstream>
#include <iomanip>
#include <regex>
#include <unistd.h>

static const char* ISTATS = "/usr/local/bin/iStats";   // change if needed

// ----------------------------- utils -----------------------------
std::string run_command(const std::string& cmd) {
    std::array<char, 256> buffer;
    std::string result;
    FILE* pipe = popen(cmd.c_str(), "r");
    if (!pipe) return "";
    while (fgets(buffer.data(), buffer.size(), pipe) != nullptr) {
        result += buffer.data();
    }
    pclose(pipe);
    if (!result.empty() && result.back() == '\n') {
        result.pop_back();
    }
    return result;
}

template <class T>
T clamp_numeric(T v, T lo, T hi) {
    return std::min(std::max(v, lo), hi);
}

int get_page_size() {
    static int size = getpagesize();
    return size;
}

// ----------------------------- hw model -----------------------------
std::string get_hw_model() {
    char buf[256];
    size_t sz = sizeof(buf);
    if (sysctlbyname("hw.model", buf, &sz, nullptr, 0) == 0) {
        return std::string(buf, sz - 1);
    }
    return "Unknown";
}

int get_logical_cores() {
    int cores = 1;
    size_t size = sizeof(cores);
    if (sysctlbyname("hw.logicalcpu", &cores, &size, nullptr, 0) != 0 || cores <= 0)
        cores = 1;
    return cores;
}

// ----------------------------- cpu -----------------------------
float get_cpu_usage_percent() {
    // Sum all process CPU usage, then normalize by logical cores
    std::string out = run_command("ps -A -o %cpu | awk '{s+=$1} END {print s}'");
    if (out.empty()) return 0.0f;
    float total = std::strtof(out.c_str(), nullptr);
    int cores = get_logical_cores();
    return total / cores;
}

// ----------------------------- memory -----------------------------
// We'll approximate "system-wide used" as active + wired + compressed.
static long extract_pages(const std::string& label) {
    std::string cmd = "vm_stat | grep \"" + label + "\" | awk '{print $3}'"; 
    std::string out = run_command(cmd);

    // Strip period and whitespace
    out.erase(std::remove(out.begin(), out.end(), '.'), out.end());
    out.erase(std::remove_if(out.begin(), out.end(), ::isspace), out.end());

    try {
        return !out.empty() ? std::stol(out) : 0L;
    } catch (const std::exception& e) {
        std::cerr << "Failed to parse page count for \"" << label << "\": " << e.what()
                  << " (raw = [" << out << "])" << std::endl;
        return 0L;
    }
}


struct MemGB {
    double used;
    double total;
};

MemGB get_memory_gb() {
    // total
    int64_t total_bytes = 0;
    size_t sz = sizeof(total_bytes);
    sysctlbyname("hw.memsize", &total_bytes, &sz, nullptr, 0);
    double total_gb = total_bytes / (1024.0 * 1024.0 * 1024.0);

    // active + wired + compressed
    long active = extract_pages("Pages active");
    long wired  = extract_pages("Pages wired down");
    long comp   = extract_pages("Pages occupied by compressor");

    double used_bytes = (active + wired + comp) * (double)get_page_size();
    double used_gb = used_bytes / (1024.0 * 1024.0 * 1024.0);

    return {used_gb, total_gb};
}

// ----------------------------- temperature -----------------------------
std::string get_temperature() {
    std::string out = run_command(std::string(ISTATS) + " cpu temp");
    std::cerr << "DEBUG: istats output = [" << out << "]\n";

    std::regex re(R"(CPU temp:\s+([\d\.]+°C))");
    std::smatch m;
    if (std::regex_search(out, m, re)) {
        return m[1];
    }
    return "N/A";
}


// ----------------------------- fan -----------------------------
struct FanInfo {
    bool present{false};
    double rpm{0.0};
    double max_rpm{0.0};
    double pct{0.0};
};


FanInfo get_fan_info(const std::string& model) {
    FanInfo info;

    // MacBook Air (M1/M2/M3, etc.) is fanless – just bail out.
    if (model.find("MacBookAir") != std::string::npos) {
        return info; // present = false
    }

    // Ask istats for fan speed lines and parse min/max/current.
    // Typical line: "Fan 0: 2160 RPM  (min: 1200 max: 7200)"
    std::string out = run_command(std::string(ISTATS) + " fan speed");
    if (out.empty()) return info;

    std::regex re(R"(Fan\s+\d+.*?(\d+(?:\.\d+)?)\s*RPM.*?min:\s*(\d+(?:\.\d+)?).*?max:\s*(\d+(?:\.\d+)?))",
                  std::regex::icase | std::regex::optimize);

    std::smatch m;
    if (std::regex_search(out, m, re) && m.size() == 4) {
        double current = std::stod(m[1]);
        double maxrpm  = std::stod(m[3]);
        if (maxrpm > 0.0) {
            info.present = true;
            info.rpm     = current;
            info.max_rpm = maxrpm;
            info.pct     = clamp_numeric((current / maxrpm) * 100.0, 0.0, 100.0);
        }
    }
    return info;
}

// ----------------------------- main -----------------------------
int main() {
    const std::string model = get_hw_model();

    float cpu = get_cpu_usage_percent();
    auto  mem = get_memory_gb();
    std::string temp = get_temperature();
    FanInfo fan = get_fan_info(model);

    // Round / format
    std::stringstream top;
    top << std::fixed << std::setprecision(1)
        << "🌡️ " << temp
        << " | 💻 " << cpu << "% CPU"
        << " | 🧠 " << std::setprecision(3) << mem.used << " / " << mem.total << " GB";

    std::stringstream drop;
    drop << "---\n"
         << "🌡️ Temp: " << temp << "\n"
         << "💻 CPU: " << std::setprecision(2) << cpu << "%\n"
         << "💾 Memory: " << std::setprecision(3) << mem.used << " / " << mem.total << " GB\n";

    if (fan.present) {
        drop << "🌀 Fan: " << std::setprecision(0) << fan.rpm << " RPM"
             << " (" << std::setprecision(1) << fan.pct << "% of " << std::setprecision(0) << fan.max_rpm << " RPM)\n";
    } 

    // clickable refresh
    drop << "Refresh Now | refresh=true\n";

    std::cout << top.str()   << std::endl;
    std::cout << drop.str();

    return 0;
}
