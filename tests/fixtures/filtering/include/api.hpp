#pragma once

#include <string>

namespace alpha::inner {

enum Mode {
    MODE_A = 0,
    MODE_B = 1,
};

class Widget {
public:
    Widget(int value);
    ~Widget();

    int value() const;
    std::string name() const;
    void set_label(const std::string& label);
};

int add(int lhs, int rhs);
int skip(int value);

} // namespace alpha::inner

namespace beta {

class Other {
public:
    Other();
    int count() const;
};

int beta_only();

} // namespace beta
