package main

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/castdrian/randomon/tools/internal/repo"
)

const upstreamURL = "https://github.com/Ajarmar/universal-pokemon-randomizer-zx"

func main() {
	destination := repo.Upstream()
	if _, err := os.Stat(filepath.Join(destination, ".git")); err != nil {
		if err := repo.Run(repo.Root(), "git", "clone", "--quiet", upstreamURL, destination); err != nil {
			repo.Fail(err)
		}
	}
	_ = repo.Run(destination, "git", "fetch", "--quiet", "origin", repo.PinnedUpstreamCommit)
	if err := repo.Run(destination, "git", "checkout", "--quiet", repo.PinnedUpstreamCommit); err != nil {
		repo.Fail(err)
	}
	fmt.Println(destination)
}
