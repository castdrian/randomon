package main

import (
	"os"

	"github.com/castdrian/randomon/tools/internal/repo"
)

func main() {
	output := repo.Path("crates", "randomon_java", "tests", "vectors")
	if len(os.Args) > 1 {
		output = os.Args[1]
	}
	build, err := os.MkdirTemp("", "randomon-java-vectors")
	if err != nil {
		repo.Fail(err)
	}
	defer os.RemoveAll(build)
	sources, err := repo.JavaSources(repo.Path("tools", "vectors", "java", "src"))
	if err != nil {
		repo.Fail(err)
	}
	exports := []string{"--add-exports", "java.base/jdk.internal.math=ALL-UNNAMED"}
	compile := append(append([]string{}, exports...), append([]string{"-encoding", "UTF-8", "-d", build}, sources...)...)
	if err := repo.Run(repo.Root(), "javac", compile...); err != nil {
		repo.Fail(err)
	}
	runtime := append(append([]string{}, exports...), "--add-opens", "java.base/java.util=ALL-UNNAMED",
		"-Duser.language=en", "-Duser.country=US", "-cp", build, "VectorGen", output)
	if err := repo.Run(repo.Root(), "java", runtime...); err != nil {
		repo.Fail(err)
	}
}
