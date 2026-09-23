#include "service.hpp"
int main() { return Service{}.get_port() == 8080 ? 0 : 1; }
