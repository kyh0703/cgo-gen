#pragma once

#include <string>

namespace safe {

class Gadget {
public:
    explicit Gadget(const std::string& name);
    ~Gadget();

    int size() const;
};

} // namespace safe
