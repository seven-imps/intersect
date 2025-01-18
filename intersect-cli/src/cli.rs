use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use intersect_core::{
    models::{Fragment, Segment, Trace},
    FragmentRecord, Identity, IndexRecord,
};
use std::{fs, path::PathBuf};

/// intersect cli
#[derive(Debug, Parser)]
#[command(name = "intersect")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    // #[command(arg_required_else_help = true)]
    // Archive {
    //     /// files to archive
    //     #[arg(required = true)]
    //     paths: Vec<PathBuf>,
    // },

    // #[command(arg_required_else_help = true)]
    // Unpack {
    //     /// file to unpack
    //     #[arg(required = true)]
    //     path: PathBuf,

    //     /// shared secret
    //     #[arg(required = true)]
    //     secret: SharedSecret,
    // },

    // #[command(arg_required_else_help = true)]
    // Validate {
    //     /// file to validate
    //     #[arg(required = true)]
    //     path: PathBuf,

    //     /// shared secret
    //     #[arg(required = true)]
    //     secret: SharedSecret,
    // },

    // #[command(arg_required_else_help = true)]
    // Publish {
    //     /// archive file
    //     #[arg(required = true)]
    //     path: PathBuf,

    //     /// public key
    //     #[arg(required = true)]
    //     public_key: PublicKey,

    //     /// private key
    //     #[arg(required = true)]
    //     private_key: CryptoKey,
    // },
    #[command(arg_required_else_help = true)]
    Lookup {
        /// trace
        #[arg(required = true, value_parser = Trace::from_str)]
        trace: Trace,
    },
    #[command(arg_required_else_help = true)]
    Post {
        /// file to post
        #[arg(required = true)]
        path: PathBuf,
        // /// public key
        // #[arg(required = true)]
        // public_key: PublicKey,

        // /// private key
        // #[arg(required = true)]
        // private_key: CryptoKey,
    },
}

pub async fn run_command(command: Cli) -> Result<()> {
    match command.command {
        // Commands::Archive { paths } => {
        //     println!("archiving {paths:?} ...");
        //     let (archive, keypair, secret) = commands::archive(paths, None, None);
        //     let pk = keypair.key;
        //     let sk = keypair.secret;
        //     println!("public key:         {pk}");
        //     println!("private key:        {sk}");
        //     println!("file shared secret: {secret}");

        //     let bytes = archive.as_bytes();
        //     fs::write("archive.isec", bytes).unwrap();
        //     println!("archive file written to archive.isec");
        // }

        // Commands::Unpack { path, secret } => {
        //     println!("unpacking {path:?} ...");
        //     let archive = commands::unpack(path, &secret);
        //     for (p, s) in archive.as_section_map() {
        //         println!("==== {} ====", p);
        //         match s {
        //             Section::Index(_index) => todo!(),
        //             Section::Fragment(frag) => println!("{}", String::from_utf8_lossy(&frag.data)),
        //         }
        //         println!("========");
        //     }
        // }

        // Commands::Validate { path, secret } => {
        //     println!("validating {path:?} ...");
        //     let is_valid = commands::validate(path, &secret);
        //     println!("archive is {}", if is_valid { "VALID" } else { "INVALID" });
        // }

        // Commands::Publish {
        //     path,
        //     public_key,
        //     private_key,
        // } => {
        //     println!("publishing {path:?} ...");
        //     commands::publish(path, &public_key, &private_key).await;
        // }
        Commands::Lookup { trace } => {
            println!("downloading trace ...");

            let mut record = IndexRecord::from_trace(&trace).await?;
            let fragment = record
                .fetch_fragment()
                .await?
                .ok_or(anyhow!("no fragment found"))?;

            let text = String::from_utf8_lossy(&fragment.fragment().data).to_string();

            println!("==== {} ====", record.meta().name());
            println!();
            println!("{text}");

            // println!("writing to {path:?}");
            // fs::write(path, archive.as_bytes()).unwrap();

            Ok(())
        }
        Commands::Post { path } => {
            let identity = Identity::random();

            let file: Vec<u8> = fs::read(path.clone())?;
            let name = Segment::new(path.file_name().unwrap().to_str().unwrap())?;

            let fragment = FragmentRecord::new(&identity, &Fragment::new(file)).await?;
            let index = IndexRecord::new(&identity, &name, Some(fragment.link()), &[]).await?;

            let trace = index.trace();
            println!("trace: {}", trace);

            Ok(())
        }
    }
}
