#pragma once

#include <cstring>

#include "iType.h"

inline void iStrCpy(char* dest, NPCSTR src) {
    std::strcpy(dest, src != nullptr ? src : "");
}
