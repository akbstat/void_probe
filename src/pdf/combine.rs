use std::{
    collections::{BTreeMap, HashMap},
    fs,
    path::{Path, PathBuf},
};

use lopdf::{Document, Object, ObjectId};
use regex::Regex;

pub struct PDFCombiner {
    outputs: HashMap<String, Vec<String>>,
    process_dir: PathBuf,
}

impl PDFCombiner {
    pub fn new(dir: &Path) -> anyhow::Result<PDFCombiner> {
        let mut outputs = HashMap::new();
        let pattern = Regex::new(r"^(?<output>(l|t|f)-.+?)_part_\d{4}.pdf").unwrap();
        for entry in fs::read_dir(dir)? {
            let entry = entry.unwrap();
            let filename = entry.file_name().to_string_lossy().to_string();
            if !filename.ends_with(".pdf") {
                continue;
            }
            if let Some(name) = pattern.captures(&filename) {
                let output = &name["output"];
                if let None = outputs.get(output) {
                    outputs.insert(output.to_string(), vec![]);
                };
                let mut parts = outputs.get(output).unwrap().to_vec();
                parts.push(filename.clone());
                outputs.insert(output.to_string(), parts.to_vec()).unwrap();
            }
        }
        Ok(PDFCombiner {
            outputs,
            process_dir: PathBuf::from(dir),
        })
    }
    pub fn combine_output(&self, dest: &Path) -> anyhow::Result<()> {
        for (output, parts) in self.outputs.clone() {
            let output_path = PathBuf::from(dest).join(format!("{}.pdf", output));
            let parts = parts
                .iter()
                .map(|f| self.process_dir.join(Path::new(f)))
                .collect::<Vec<PathBuf>>();
            combine_one_output(&parts, output_path.as_path())?;
            parts.iter().for_each(|f| fs::remove_file(f).unwrap());
        }

        Ok(())
    }
}

fn combine_one_output(source: &[PathBuf], dest: &Path) -> anyhow::Result<()> {
    let mut document = Document::with_version("1.7");
    let mut documents = vec![];
    for f in source {
        let doc = Document::load(f)?;
        documents.push(doc);
    }

    // Define a starting max_id (will be used as start index for object_ids)
    let mut max_id = 1;
    let mut pagenum = 1;
    // Collect all Documents Objects grouped by a map
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();

    for mut doc in documents {
        let mut first = false;
        doc.renumber_objects_with(max_id);

        max_id = doc.max_id + 1;

        documents_pages.extend(
            doc.get_pages()
                .into_iter()
                .map(|(_, object_id)| {
                    if !first {
                        first = true;
                        pagenum += 1;
                    }

                    (object_id, doc.get_object(object_id).unwrap().to_owned())
                })
                .collect::<BTreeMap<ObjectId, Object>>(),
        );
        documents_objects.extend(doc.objects);
    }

    // Catalog and Pages are mandatory
    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    // Process all objects except "Page" type
    for (object_id, object) in documents_objects.iter() {
        // We have to ignore "Page" (as are processed later), "Outlines" and "Outline" objects
        // All other objects should be collected and inserted into the main Document
        match object.type_name().unwrap_or("") {
            "Catalog" => {
                // Collect a first "Catalog" object and use it for the future "Pages"
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object {
                        id
                    } else {
                        *object_id
                    },
                    object.clone(),
                ));
            }
            "Pages" => {
                // Collect and update a first "Pages" object and use it for the future "Catalog"
                // We have also to merge all dictionaries of the old and the new "Pages" object
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref object)) = pages_object {
                        if let Ok(old_dictionary) = object.as_dict() {
                            dictionary.extend(old_dictionary);
                        }
                    }

                    pages_object = Some((
                        if let Some((id, _)) = pages_object {
                            id
                        } else {
                            *object_id
                        },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            "Page" => {}     // Ignored, processed later and separately
            "Outlines" => {} // Ignored, not supported yet
            "Outline" => {}  // Ignored, not supported yet
            _ => {
                document.objects.insert(*object_id, object.clone());
            }
        }
    }

    // If no "Pages" object found abort
    if pages_object.is_none() {
        // println!("Pages root not found.");
        return Ok(());
    }

    // Iterate over all "Page" objects and collect into the parent "Pages" created before
    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.as_ref().unwrap().0);

            document
                .objects
                .insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    // If no "Catalog" found abort
    if catalog_object.is_none() {
        // println!("Catalog root not found.");

        return Ok(());
    }

    let catalog_object = catalog_object.unwrap();
    let pages_object = pages_object.unwrap();

    // Build a new "Pages" with updated fields
    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();

        // Set new pages count
        dictionary.set("Count", documents_pages.len() as u32);

        // Set new "Kids" list (collected from documents pages) for "Pages"
        dictionary.set(
            "Kids",
            documents_pages
                .into_iter()
                .map(|(object_id, _)| Object::Reference(object_id))
                .collect::<Vec<_>>(),
        );

        document
            .objects
            .insert(pages_object.0, Object::Dictionary(dictionary));
    }

    // Build a new "Catalog" with updated fields
    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines"); // Outlines not supported in merged PDFs

        document
            .objects
            .insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);

    // Update the max internal ID as wasn't updated before due to direct objects insertion
    document.max_id = document.objects.len() as u32;

    // Reorder all new Document objects
    document.renumber_objects();

    //Set any Bookmarks to the First child if they are not set to a page
    document.adjust_zero_pages();

    //Set all bookmarks to the PDF Object tree then set the Outlines to the Bookmark content map.
    if let Some(n) = document.build_outline() {
        if let Ok(x) = document.get_object_mut(catalog_object.0) {
            if let Object::Dictionary(ref mut dict) = x {
                dict.set("Outlines", Object::Reference(n));
            }
        }
    }

    document.compress();
    document.save(dest)?;
    Ok(())
}

#[cfg(test)]
mod test_pdf_combine {
    use super::*;
    #[test]
    fn pdf_combine_test() {
        let dir = Path::new(r"D:\Studies\ak112\303\stats\CSR\product\output\.temp");
        let dest = Path::new(r"D:\Studies\ak112\303\stats\CSR\product\output\.temp");
        let combiner = PDFCombiner::new(dir).unwrap();
        combiner.combine_output(dest).unwrap();
    }
}
