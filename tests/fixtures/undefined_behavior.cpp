#include <limits>

int main() {
    volatile int value = std::numeric_limits<int>::max();
    return value + 1;
}
