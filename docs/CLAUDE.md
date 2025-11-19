# CLAUDE.md - AI Assistant Guide for NixOS Artifacts Documentation

## Project Context

You are working on the **NixOS Artifacts Documentation**, an Antora-based
documentation site for the NixOS Artifacts framework. This documentation
explains how to unify artifacts and secrets in NixOS flakes through a
standardized interface with pluggable backends.

## Core Purpose

This documentation site serves as:

- User-facing documentation for the NixOS Artifacts framework
- Getting started guides and tutorials
- Technical reference for options and backends
- Workflow diagrams and architecture explanations

## Project Structure

```
docs/
├── README.md                    # Build instructions and structure overview
├── antora.yml                   # Antora component descriptor
├── ui-bundle.zip                # Antora UI theme bundle
└── modules/
    └── ROOT/
        ├── nav.adoc             # Navigation structure
        ├── images/              # Architecture diagrams
        │   ├── architecture.graphml
        │   └── architecture.svg
        ├── pages/               # Main documentation pages
        │   ├── index.adoc                         # Landing page
        │   ├── what-is-nixos-artifacts.adoc       # Overview and concepts
        │   ├── artifact-definition-example.adoc   # How to define artifacts
        │   ├── generate-artifacts-cli.adoc        # CLI usage guide
        │   ├── defining-backends.adoc             # Backend development guide
        │   ├── how-to-use-a-backend.adoc         # Using backends
        │   ├── artifacts-workflow-diagrams.adoc   # Workflow visualization
        │   └── options.adoc                       # Options reference
        └── partials/            # Reusable documentation fragments
            ├── artifacts-input-example.adoc
            ├── artifact-store-example-ssh.adoc
            ├── artifact-cli-configuration.adoc
            └── workflow-loop-diagram.mermaid
```

## Documentation Format

- **Format**: AsciiDoc (`.adoc` files)
- **Build tool**: Antora
- **Navigation**: Defined in `modules/ROOT/nav.adoc`
- **Partials**: Reusable content fragments in `partials/` directory

## Building the Documentation

```bash
# Build the documentation site
nix run .#build-docs

# Output is generated to docs/public/
# Open docs/public/index.html in browser
```

## Key Documentation Sections

1. **What is NixOS Artifacts** - Core concepts, architecture, and design
   philosophy
2. **Getting Started** - Three-step setup: flake setup, CLI configuration,
   artifact definition
3. **Artifact Definition** - How to declare artifacts using NixOS options
4. **CLI Usage** - Commands for generating and rotating artifacts
5. **Backend Integration** - How to use and define backends
6. **Workflow Diagrams** - Visual representation of the generation/rotation flow
7. **Options Reference** - Complete reference of available options

## Development Guidelines

### When Making Changes

1. **Use AsciiDoc syntax** - Not Markdown
2. **Update navigation** - Modify `nav.adoc` if adding new pages
3. **Keep partials DRY** - Extract reusable examples to `partials/`
4. **Reference other pages** - Use `xref:page-name.adoc[Link Text]` for internal
   links
5. **Test builds locally** - Run `nix run .#build-docs` before committing
6. **Maintain consistency** - Follow existing style and structure patterns

### AsciiDoc Conventions

- Headers: `= Title`, `== Section`, `=== Subsection`
- Cross-references: `xref:page-name.adoc[Link text]`
- External links: `https://example.com[Link text]`
- Code blocks: Use `[source,nix]` for syntax highlighting
- Includes: `include::partial$filename.adoc[]`
- Images: `image::filename.svg[Alt text]`
- Warnings: `WARNING: Text here`
- Notes: `NOTE: Text here`

### Content Organization

- **Pages**: Complete standalone documentation articles
- **Partials**: Reusable code examples and fragments
- **Images**: Architecture diagrams and illustrations
- **Navigation**: Hierarchical menu structure in `nav.adoc`

## Key Concepts Explained in Docs

- **Artifacts**: Named bundles of generated files (secrets, keys, configs)
- **Backends**: Pluggable storage engines (agenix, sops-nix, etc.)
- **Store**: High-level NixOS option tree for artifact declarations
- **Generator**: Tool that produces artifact files from prompts
- **CLI**: Command-line tool for orchestrating generation and rotation

## Common Tasks

### Adding a New Documentation Page

1. Create `.adoc` file in `modules/ROOT/pages/`
2. Add entry to `modules/ROOT/nav.adoc`
3. Reference from existing pages using `xref:`
4. Build to verify navigation works

### Adding a Reusable Example

1. Create `.adoc` file in `modules/ROOT/partials/`
2. Include in pages using `include::partial$filename.adoc[]`

### Updating Architecture Diagrams

- Edit `.graphml` files with yEd or similar tool
- Export to `.svg` format
- Place both in `modules/ROOT/images/`

## Target Audience

- NixOS users familiar with flakes
- System administrators managing secrets
- Backend developers extending the framework
- Contributors to the NixOS Artifacts project

## Documentation Status

⚠️ **Experimental Project**: Documentation emphasizes that interfaces and
options may change. All pages should maintain appropriate warnings about
stability.

## Quick Reference

- Primary format: AsciiDoc
- Build command: `nix run .#build-docs`
- Output directory: `docs/public/`
- Component name: `nixos-artifacts`
- Version: `latest`

## External References

The documentation references:

- `nixos-artifacts-agenix` backend documentation (separate Antora component)
- Clan vars documentation (inspiration source)
- NixOS PR #370444 (related work)
- agenix-rekey project (inspiration source)
