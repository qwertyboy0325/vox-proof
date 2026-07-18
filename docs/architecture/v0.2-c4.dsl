// VoxProof v0.2 C4 Architecture Draft — Structurizr DSL
// Status: draft / proposed — synchronized with architecture/v0.2-c4-architecture.md
// Renderer validation: pending (no Structurizr tool in repository; syntax manually inspected)
//
// C4 rule: linked Rust library is a component boundary inside executables, not a Level 2 container.

workspace "VoxProof v0.2 C4 Draft" "Proposed v0.2 architecture planning artifact. Does not establish v0.2 scope." {

    !identifiers hierarchical

    model {
        reviewer = person "Transcript Reviewer" "Opens a transcript; inspects evidence; listens to optional local media; explicitly accepts, rejects, or authors a correction; exports reviewed output." {
            tags "Person"
        }

        localFilesystem = softwareSystem "Local filesystem / OS environment" "Hosts processes; stores SRT, session terms, reviewed output, logs, summaries, session store, and optional media." {
            tags "External"
        }

        sourceSrt = softwareSystem "Source SRT" "Existing ASR transcript input file." {
            tags "External"
        }

        sessionTerms = softwareSystem "Session terms file" "Provisional session-scoped term input." {
            tags "External"
        }

        reviewedSrt = softwareSystem "Reviewed SRT" "Derived reviewed transcript output." {
            tags "External"
        }

        decisionLog = softwareSystem "Decision log" "Deterministic exported representation of ReviewLedger events. Not the source of authority." {
            tags "External"
        }

        sessionSummary = softwareSystem "Session summary" "Human-readable session artifact." {
            tags "External"
        }

        optionalMedia = softwareSystem "Optional local media" "Optional audio/video for human listening; no correction authority." {
            tags "External" "ProposedV02"
        }

        voxproof = softwareSystem "VoxProof" "Local-first, evidence-backed post-ASR transcript review system." {
            tags "EstablishedV01"

            voxCli = container "VoxProof CLI" "Primary product executable: parse, review, compare, evaluate, review-experiment. Links the Rust library as an in-process component boundary." "Rust" {
                tags "EstablishedV01"

                cliReview = component "Interactive Review Loop" "Prompts reviewer; records decisions via ReviewLedger; triggers export." "Rust" {
                    tags "EstablishedV01"
                }
                cliIO = component "Filesystem I/O" "Reads/writes SRT, terms, logs, summaries, JSON reports." "Rust" {
                    tags "EstablishedV01"
                }
                cliSrtParser = component "SRT Parser" "Parses SRT into transcript model." "Rust module srt" {
                    tags "EstablishedV01"
                }
                cliTranscript = component "Transcript / Source Model" "Source model and MD-001 revision id." "Rust module transcript" {
                    tags "EstablishedV01"
                }
                cliAnchor = component "Anchors" "Byte-range anchors into source text." "Rust module anchor" {
                    tags "EstablishedV01"
                }
                cliTerms = component "Session Terms Import" "Parses provisional session terms." "Rust module session_terms" {
                    tags "EstablishedV01"
                }
                cliAnalysis = component "Analysis Run & Identity" "Configuration snapshot for detectors." "Rust module analysis" {
                    tags "EstablishedV01"
                }
                cliDetectors = component "ReviewCase / Candidate Generation" "Exact alias, observed error, ASCII-Latin phonetic via pipeline." "Rust modules candidate, phonetic, pipeline" {
                    tags "EstablishedV01"
                }
                cliLedger = component "Review Ledger" "Append-only authoritative decisions." "Rust module review" {
                    tags "EstablishedV01"
                }
                cliMaterializer = component "Reviewed Output Materializer" "derive_reviewed_srt fail-closed path." "Rust module reviewed_output" {
                    tags "EstablishedV01"
                }
                cliValidation = component "Strict Validation / Refusal" "Revision, overlap, and decision checks." "Rust" {
                    tags "EstablishedV01"
                }
                cliSessionLog = component "Session Log / Summary" "Renders decision log and session summary exports." "Rust modules session_log, session_summary" {
                    tags "EstablishedV01"
                }
                cliCompareEval = component "Compare / Evaluate Tooling" "Strict skeleton calibration reports." "Rust modules calibration, calibration_join" {
                    tags "EstablishedV01"
                }
                cliExperimental = component "Experimental Retrieval / Ranking" "Non-canonical sidecar; cannot write authoritative decisions." "Rust modules experimental_retrieval, experimental_ranking" {
                    tags "EstablishedV01"
                }
            }

            desktopApp = container "VoxProof Desktop Application" "Proposed v0.2 desktop executable/process. Technology: TBD. Contains UI, application orchestration, adapters, and linked core semantics." "Technology: TBD" {
                tags "ProposedV02" "TBD"

                reviewUI = component "Review UI" "Presents ReviewCases; submits human intent; not authoritative." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }
                appAdapter = component "Desktop/Application Adapter" "Maps UI intent to application use cases." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }
                orchestrator = component "Application Orchestrator" "Sequences use cases; not part of the core domain." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }
                openUC = component "Transcript Import / Open Use Case" "Opens source transcript and session terms." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }
                sessionUC = component "Review Session Use Cases" "Drives review presentation and case navigation." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }
                decisionUC = component "Decision Recording Use Case" "Forwards validated human intent to ReviewLedger." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }
                materializeUC = component "Materialization / Export Use Case" "Invokes canonical materializer and export paths." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }
                sessionSer = component "Session Serialization Adapter" "Save/load versioned session state; not core domain." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }
                fsAdapter = component "Filesystem Adapter" "Path import/export infrastructure." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }
                mediaAdapter = component "Optional Media Playback Adapter" "Sensory evidence only; no correction authority." "Technology: TBD" {
                    tags "ProposedV02" "TBD"
                }

                desktopSrtParser = component "SRT Parser" "Linked Rust module srt; reused in-process." "Rust module srt" {
                    tags "EstablishedV01"
                }
                desktopTranscript = component "Transcript / Source Model" "Linked Rust module transcript; MD-001." "Rust module transcript" {
                    tags "EstablishedV01"
                }
                desktopAnchor = component "Anchors" "Linked Rust module anchor." "Rust module anchor" {
                    tags "EstablishedV01"
                }
                desktopTerms = component "Session Terms Import" "Linked Rust module session_terms." "Rust module session_terms" {
                    tags "EstablishedV01"
                }
                desktopAnalysis = component "Analysis Run & Identity" "Linked Rust module analysis; MD-004." "Rust module analysis" {
                    tags "EstablishedV01"
                }
                desktopDetectors = component "ReviewCase / Candidate Generation" "Linked candidate, phonetic, pipeline." "Rust modules candidate, phonetic, pipeline" {
                    tags "EstablishedV01"
                }
                desktopLedger = component "Review Ledger" "Linked Rust module review; MD-002." "Rust module review" {
                    tags "EstablishedV01"
                }
                desktopMaterializer = component "Reviewed Output Materializer" "Linked Rust module reviewed_output; MD-003." "Rust module reviewed_output" {
                    tags "EstablishedV01"
                }
                desktopValidation = component "Strict Validation / Refusal" "Linked materialization and ledger checks." "Rust" {
                    tags "EstablishedV01"
                }
                desktopSessionLog = component "Session Log / Summary" "Linked session_log and session_summary." "Rust modules session_log, session_summary" {
                    tags "EstablishedV01"
                }
                desktopCompareEval = component "Compare / Evaluate Tooling" "Linked calibration tooling; not default product flow." "Rust modules calibration, calibration_join" {
                    tags "EstablishedV01"
                }
                desktopExperimental = component "Experimental Retrieval / Ranking" "Linked experimental modules; non-authoritative." "Rust modules experimental_retrieval, experimental_ranking" {
                    tags "EstablishedV01"
                }
            }

            sessionStore = container "Versioned Local Session Store" "Proposed local persisted session resource. Technology and format: TBD. Not an active service." "Technology and format: TBD" {
                tags "ProposedV02" "TBD" "DataStore"
            }
        }

        // System Context relationships
        reviewer -> voxproof "Reviews transcripts and authorizes corrections"
        reviewer -> optionalMedia "Listens for sensory evidence" "Optional; proposed v0.2"
        voxproof -> sourceSrt "Reads"
        voxproof -> sessionTerms "Reads"
        voxproof -> reviewedSrt "Writes via canonical materializer"
        voxproof -> decisionLog "Exports decision log"
        voxproof -> sessionSummary "Writes"
        voxproof -> localFilesystem "Runs on / reads and writes"
        optionalMedia -> localFilesystem "Stored on"
        sourceSrt -> localFilesystem "Stored on"

        // v0.1 container relationships
        reviewer -> voxproof.voxCli "Runs commands, enters decisions"
        voxproof.voxCli -> sourceSrt "Reads"
        voxproof.voxCli -> sessionTerms "Reads"
        voxproof.voxCli -> reviewedSrt "Writes via canonical materializer"
        voxproof.voxCli -> decisionLog "Exports"
        voxproof.voxCli -> sessionSummary "Writes"
        voxproof.voxCli -> localFilesystem "Runs on / I/O"
        voxproof.voxCli.cliReview -> voxproof.voxCli.cliDetectors "run_term_review"
        voxproof.voxCli.cliReview -> voxproof.voxCli.cliLedger "record_decision"
        voxproof.voxCli.cliReview -> voxproof.voxCli.cliMaterializer "derive_reviewed_srt"
        voxproof.voxCli.cliMaterializer -> voxproof.voxCli.cliLedger "Reads accepted decisions"
        voxproof.voxCli.cliMaterializer -> voxproof.voxCli.cliTranscript "Reads immutable source"
        voxproof.voxCli.cliIO -> localFilesystem "Reads/writes artifacts"

        // v0.2 proposed container relationships
        reviewer -> voxproof.desktopApp "Inspects and decides" "Proposed"
        reviewer -> voxproof.voxCli "May continue using"
        voxproof.desktopApp -> sourceSrt "Reads" "Proposed"
        voxproof.desktopApp -> reviewedSrt "Writes via canonical materializer" "Proposed"
        voxproof.desktopApp -> decisionLog "Exports" "Proposed"
        voxproof.desktopApp -> sessionSummary "Writes" "Proposed"
        voxproof.desktopApp -> voxproof.sessionStore "Save/load session" "Proposed"
        voxproof.desktopApp -> optionalMedia "Plays for human judgment" "Proposed; no correction authority"
        voxproof.desktopApp -> localFilesystem "Runs on / I/O" "Proposed"
        voxproof.sessionStore -> localFilesystem "Stored on" "Proposed"

        // Desktop Application component relationships (owned by desktopApp only)
        voxproof.desktopApp.reviewUI -> voxproof.desktopApp.appAdapter "Submits human intent"
        voxproof.desktopApp.appAdapter -> voxproof.desktopApp.orchestrator "Forwards validated intent"
        voxproof.desktopApp.orchestrator -> voxproof.desktopApp.openUC "Open transcript"
        voxproof.desktopApp.orchestrator -> voxproof.desktopApp.sessionUC "Drive review session"
        voxproof.desktopApp.orchestrator -> voxproof.desktopApp.decisionUC "Record decision"
        voxproof.desktopApp.orchestrator -> voxproof.desktopApp.materializeUC "Materialize/export"
        voxproof.desktopApp.openUC -> voxproof.desktopApp.desktopSrtParser "Parse"
        voxproof.desktopApp.openUC -> voxproof.desktopApp.desktopTranscript "Build source model"
        voxproof.desktopApp.sessionUC -> voxproof.desktopApp.desktopDetectors "Produce ReviewCases"
        voxproof.desktopApp.decisionUC -> voxproof.desktopApp.desktopLedger "record_decision"
        voxproof.desktopApp.materializeUC -> voxproof.desktopApp.desktopMaterializer "derive_reviewed_srt"
        voxproof.desktopApp.materializeUC -> voxproof.desktopApp.desktopValidation "Fail closed on invalid input"
        voxproof.desktopApp.orchestrator -> voxproof.desktopApp.sessionSer "Save/load session"
        voxproof.desktopApp.orchestrator -> voxproof.desktopApp.fsAdapter "Import/export paths"
        voxproof.desktopApp.orchestrator -> voxproof.desktopApp.mediaAdapter "Optional playback"
        voxproof.desktopApp.desktopMaterializer -> voxproof.desktopApp.desktopLedger "Reads accepted decisions"
        voxproof.desktopApp.desktopMaterializer -> voxproof.desktopApp.desktopTranscript "Reads immutable source"
    }

    views {
        systemContext voxproof "SystemContext" {
            include reviewer
            include voxproof
            include sourceSrt
            include sessionTerms
            include reviewedSrt
            include decisionLog
            include sessionSummary
            include localFilesystem
            include optionalMedia
            autoLayout
        }

        container voxproof "V01Containers" {
            include reviewer
            include voxproof.voxCli
            include sourceSrt
            include sessionTerms
            include reviewedSrt
            include decisionLog
            include sessionSummary
            include localFilesystem
            autoLayout
        }

        container voxproof "V02ProposedContainers" {
            include reviewer
            include voxproof.voxCli
            include voxproof.desktopApp
            include voxproof.sessionStore
            include sourceSrt
            include reviewedSrt
            include decisionLog
            include sessionSummary
            include optionalMedia
            include localFilesystem
            autoLayout
        }

        component voxproof.desktopApp "CoreApplicationComponents" {
            include *
            autoLayout
        }

        styles {
            element "Person" {
                shape Person
                background #08427b
                color #ffffff
            }
            element "EstablishedV01" {
                background #1168bd
                color #ffffff
            }
            element "ProposedV02" {
                background #999999
                color #ffffff
                stroke #666666
                strokeWidth 2
                opacity 70
            }
            element "External" {
                background #999999
                color #ffffff
            }
            element "TBD" {
                stroke #ff6600
                strokeWidth 3
                opacity 50
            }
            element "DataStore" {
                shape Cylinder
            }
        }
    }

}
