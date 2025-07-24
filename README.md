# Sysmon

A lightweight and secure temperature monitoring tool for macOS, designed with privacy and efficiency in mind.

---

**Author:** Kushal Dudipala  
**GitHub:** [kushaldudipala](https://github.com/kushaldudipala)  

---
## Highlights
- **No root / sudo required.**
- **No kernel extensions or SMC writes:** Only reads user-visible stats exposed by the OS or iStats. (iStat Menus also operates within SMC constraints. [Bjango])
- **Full binary paths:** Uses absolute paths (e.g., `/opt/homebrew/bin/istats`) to prevent PATH hijacking.
- **Safe parsing:** All numbers from shell commands are range-checked and parsed safely.
- **Minimal resource usage:** SwiftBar executes your binary every _N_ seconds and exits, resulting in almost zero idle footprint.

