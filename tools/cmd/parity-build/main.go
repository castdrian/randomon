package main

import (
	"fmt"
	"os"

	"github.com/castdrian/randomon/tools/internal/repo"
)

func main() {
	classes := repo.Path("target", "parity", "classes")
	if err := os.RemoveAll(classes); err != nil {
		repo.Fail(err)
	}
	if err := os.MkdirAll(classes, 0o755); err != nil {
		repo.Fail(err)
	}
	err := repo.Run(repo.Root(), "javac", "-nowarn", "-encoding", "UTF-8", "-Xlint:none",
		"-sourcepath", repo.UpstreamSource(),
		"-d", classes,
		repo.Path("tools", "parity", "java", "ParityRunner.java"))
	if err != nil {
		repo.Fail(err)
	}
	fmt.Println("Parity runner compiled into", classes)
}
