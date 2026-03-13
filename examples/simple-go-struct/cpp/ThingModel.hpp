#pragma once

#include <string>

class ThingModel {
public:
    ThingModel();
    ~ThingModel();

    int GetValue() const;
    void SetValue(int value);

    const char* GetName() const;
    void SetName(const char* name);

private:
    int value_;
    std::string name_;
};
