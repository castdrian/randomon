package main

import (
	"fmt"
	"os"
	"regexp"

	"github.com/castdrian/randomon/tools/internal/repo"
)

var workspaceVersion = regexp.MustCompile(`(?s)(\[workspace\.package\][^\[]*?\nversion = ")[^"]+(")`)

func main() {
	if len(os.Args) != 2 {
		repo.Fail(fmt.Errorf("usage: set-version <version>"))
	}
	version := os.Args[1]
	manifest := repo.Path("Cargo.toml")
	contents, err := os.ReadFile(manifest)
	if err != nil {
		repo.Fail(err)
	}
	if !workspaceVersion.Match(contents) {
		repo.Fail(fmt.Errorf("no [workspace.package] version found in %s", manifest))
	}
	updated := workspaceVersion.ReplaceAll(contents, []byte("${1}"+version+"${2}"))
	if err := os.WriteFile(manifest, updated, 0o644); err != nil {
		repo.Fail(err)
	}
	if err := repo.Run(repo.Root(), "cargo", "update", "--workspace", "--offline"); err != nil {
		if err := repo.Run(repo.Root(), "cargo", "update", "--workspace"); err != nil {
			repo.Fail(err)
		}
	}
	fmt.Println("version =", version)
}
