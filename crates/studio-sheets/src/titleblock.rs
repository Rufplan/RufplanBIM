//! The title block's editable fields (ADR-111): double-clicking a sheet's title block opens
//! them, as Revit edits a title block's labels in place. The project and the architect's
//! firm are Project Information (ADR-084), the license line is the project's, drawn and
//! checked by are each sheet's own, kept as parameters (no file-format change).

use serde::{Deserialize, Serialize};
use studio_core::params::ParamValue;
use studio_core::{CoreError, CoreResult, Document, ElementData, ElementId};
use studio_geom::Pt;
use ts_rs::TS;

/// ProjectInfo's parameter with the architect's license line.
pub const LICENSE_KEY: &str = "rufplan.titleblock.license";
/// A sheet's parameter with who drew and checked it.
pub const SIGN_KEY: &str = "rufplan.titleblock.sign";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct License {
    pub number: String,
    pub renews: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct SignedBy {
    pub drawn: String,
    pub checked: String,
}

/// Everything the title block prints that isn't worked out (date, scale, issues).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TitleBlockFields {
    pub sheet_number: String,
    pub sheet_name: String,
    pub project_name: String,
    pub project_number: String,
    /// The owner (the client's company).
    pub client: String,
    pub street: String,
    pub city: String,
    pub state: String,
    pub zip: String,
    pub firm: String,
    pub firm_address: String,
    pub firm_phone: String,
    pub firm_email: String,
    pub firm_website: String,
    pub license: License,
    pub signed: SignedBy,
}

fn json<T: for<'a> Deserialize<'a> + Default>(doc: &Document, id: ElementId, key: &str) -> T {
    match doc.param(id, key) {
        Some(ParamValue::Text(s)) => serde_json::from_str(s).unwrap_or_default(),
        _ => T::default(),
    }
}

/// The project's license line.
pub fn license(doc: &Document) -> License {
    studio_core::ops::project_info(doc).map_or_else(License::default, |i| json(doc, i, LICENSE_KEY))
}

/// Who drew and checked a sheet.
pub fn signed(doc: &Document, sheet: ElementId) -> SignedBy {
    json(doc, sheet, SIGN_KEY)
}

/// The architect on the team (not the landscape architect).
pub fn architect_index(d: &studio_core::project::ProjectDetails) -> Option<usize> {
    d.team.iter().position(|t| {
        let s = t.discipline.to_lowercase();
        s.contains("architect") && !s.contains("landscape")
    })
}

/// A sheet's title block fields.
pub fn fields(doc: &Document, sheet: ElementId) -> CoreResult<TitleBlockFields> {
    let ElementData::Sheet { number, name, .. } = doc.data(sheet)? else {
        return Err(CoreError::Invalid("that isn't a sheet".into()));
    };
    let (ident, d) = studio_core::project::get(doc)?;
    let firm = architect_index(&d)
        .map(|i| d.team[i].contact.clone())
        .unwrap_or_default();
    Ok(TitleBlockFields {
        sheet_number: number.clone(),
        sheet_name: name.clone(),
        project_name: ident.name,
        project_number: ident.number,
        client: if d.client.company.trim().is_empty() {
            d.client.name.clone()
        } else {
            d.client.company.clone()
        },
        street: d.location.street.clone(),
        city: d.location.city.clone(),
        state: d.location.state.clone(),
        zip: d.location.zip.clone(),
        firm: firm.company,
        firm_address: firm.address,
        firm_phone: firm.phone,
        firm_email: firm.email,
        firm_website: firm.website,
        license: license(doc),
        signed: signed(doc, sheet),
    })
}

/// Saves a sheet's title block fields in one undoable step.
pub fn set_fields(doc: &mut Document, sheet: ElementId, f: &TitleBlockFields) -> CoreResult<()> {
    if !matches!(doc.data(sheet)?, ElementData::Sheet { .. }) {
        return Err(CoreError::Invalid("that isn't a sheet".into()));
    }
    let number = f.sheet_number.trim();
    if number.is_empty() {
        return Err(CoreError::Invalid("the sheet needs a number".into()));
    }
    let taken = doc.iter().any(|e| {
        e.id != sheet
            && matches!(&e.data, ElementData::Sheet { number: n, .. } if n.trim() == number)
    });
    if taken {
        return Err(CoreError::Invalid(format!("sheet {number} already exists")));
    }
    let info = studio_core::ops::project_info(doc)
        .ok_or_else(|| CoreError::Invalid("project information is missing".into()))?;
    let (_, mut d) = studio_core::project::get(doc)?;
    if d.client.company.trim().is_empty() && !d.client.name.trim().is_empty() {
        d.client.name = f.client.trim().into();
    } else {
        d.client.company = f.client.trim().into();
    }
    d.location.street = f.street.trim().into();
    d.location.city = f.city.trim().into();
    d.location.state = f.state.trim().into();
    d.location.zip = f.zip.trim().into();
    let i = match architect_index(&d) {
        Some(i) => i,
        None => {
            d.team.insert(
                0,
                studio_core::project::TeamMember {
                    discipline: "Architect".into(),
                    ..Default::default()
                },
            );
            0
        }
    };
    let c = &mut d.team[i].contact;
    c.company = f.firm.trim().into();
    c.address = f.firm_address.trim().into();
    c.phone = f.firm_phone.trim().into();
    c.email = f.firm_email.trim().into();
    c.website = f.firm_website.trim().into();

    let mark = doc.undo_depth();
    studio_core::project::set(doc, &f.project_name, &f.project_number, d)?;
    let text = |v: String| Some(ParamValue::Text(v));
    let license =
        serde_json::to_string(&f.license).map_err(|e| CoreError::Invalid(e.to_string()))?;
    let signed = serde_json::to_string(&f.signed).map_err(|e| CoreError::Invalid(e.to_string()))?;
    let (name, number) = (f.sheet_name.trim().to_string(), number.to_string());
    let done = doc.transact("Edit Title Block", |tx| {
        tx.modify(sheet, |e| {
            if let ElementData::Sheet {
                number: n, name: m, ..
            } = e
            {
                *n = number.clone();
                *m = name.clone();
            }
        })?;
        tx.set_param(info, LICENSE_KEY, text(license.clone()))?;
        tx.set_param(sheet, SIGN_KEY, text(signed.clone()))
    });
    if let Err(e) = done {
        let _ = doc.undo();
        return Err(e);
    }
    doc.merge_undo(mark, "Edit Title Block");
    Ok(())
}

/// Whether paper point `at` lies on the sheet's title block strip.
pub fn hit(doc: &Document, sheet: ElementId, at: Pt) -> bool {
    let Ok(ElementData::Sheet { size, .. }) = doc.data(sheet) else {
        return false;
    };
    let (w, h) = size.mm();
    let (_, m) = crate::sheet::margins(*size);
    let x0 = w - m - crate::sheet::title_block_width(*size);
    at.x >= x0 && at.x <= w - m && at.y >= m && at.y <= h - m
}

#[cfg(test)]
mod tests {
    use super::*;
    use studio_core::SheetSize;

    fn sheet(doc: &mut Document, number: &str) -> ElementId {
        let mut id = None;
        doc.transact("Sheet", |tx| {
            id = Some(tx.insert(ElementData::Sheet {
                number: number.into(),
                name: "Floor Plan".into(),
                size: SheetSize::ArchD,
                stages: vec![],
            }));
            Ok(())
        })
        .unwrap();
        id.unwrap()
    }

    #[test]
    fn title_block_fields_save_in_one_step_and_print() {
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let a = sheet(&mut doc, "A-101");
        sheet(&mut doc, "A-102");
        let mut f = fields(&doc, a).unwrap();
        let before = f.project_name.clone();
        assert_eq!(f.sheet_number, "A-101");
        f.project_name = "Hale Residence".into();
        f.firm = "Kerr Architects".into();
        f.street = "100 Main St".into();
        f.license = License {
            number: "C-12345".into(),
            renews: "06/30/2028".into(),
        };
        f.signed = SignedBy {
            drawn: "RK".into(),
            checked: "JD".into(),
        };
        f.sheet_name = "First Floor Plan".into();
        let depth = doc.undo_depth();
        set_fields(&mut doc, a, &f).unwrap();
        assert_eq!(doc.undo_depth(), depth + 1);
        let back = fields(&doc, a).unwrap();
        assert_eq!(back.project_name, "Hale Residence");
        assert_eq!(back.firm, "Kerr Architects");
        assert_eq!(back.license.number, "C-12345");
        assert_eq!(back.signed.checked, "JD");
        assert_eq!(back.sheet_name, "First Floor Plan");

        let dl = crate::sheet::sheet_display_list(&doc, a, "2026-10-04").unwrap();
        let texts: Vec<String> = dl
            .items
            .iter()
            .filter_map(|it| match &it.prim {
                studio_views::Prim::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        for want in ["KERR ARCHITECTS", "HALE RESIDENCE", "RK", "JD"] {
            assert!(texts.iter().any(|t| t == want), "{want} in {texts:?}");
        }
        assert!(texts
            .iter()
            .any(|t| t.contains("C-12345") && t.contains("06/30/2028")));

        // One undo takes it all back; a taken number is refused.
        doc.undo().unwrap();
        assert_eq!(fields(&doc, a).unwrap().project_name, before);
        f.sheet_number = "A-102".into();
        assert!(set_fields(&mut doc, a, &f).is_err());
    }

    #[test]
    fn the_title_block_strip_is_hit_on_the_right() {
        let mut doc = Document::new();
        studio_core::ops::seed_default_project(&mut doc).unwrap();
        let a = sheet(&mut doc, "A-101");
        let (w, h) = SheetSize::ArchD.mm();
        assert!(hit(&doc, a, Pt::new(w - 40.0, h / 2.0)));
        assert!(!hit(&doc, a, Pt::new(w / 2.0, h / 2.0)));
    }
}
