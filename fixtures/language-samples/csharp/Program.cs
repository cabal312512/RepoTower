using Toy.Services;
namespace Toy;
public static class Program { public static int Main() => new Service().Port() == 8080 ? 0 : 1; }
