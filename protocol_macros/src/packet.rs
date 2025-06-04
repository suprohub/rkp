use std::fs::File;
use std::io::Write;

use heck::{ToShoutySnakeCase, ToSnakeCase};
use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, DeriveInput, Error, Expr, LitInt, LitStr, Result, parse_quote, parse2};

use crate::add_trait_bounds;

pub(super) fn derive_packet(item: TokenStream) -> Result<TokenStream> {
    let mut input = parse2::<DeriveInput>(item)?;

    let packet_attr = parse_packet_helper_attr(&input.attrs)?.unwrap_or_default();

    let name = &input.ident;

    let name_orig = name.to_string();

    add_trait_bounds(&mut input.generics, quote!(::std::fmt::Debug));

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let side = if let Some(side_attr) = packet_attr.side {
        side_attr
    } else if name_orig.starts_with("C") {
        parse_quote!(::protocol::PacketSide::Clientbound)
    } else if name_orig.starts_with("S") {
        parse_quote!(::protocol::PacketSide::Serverbound)
    } else {
        return Err(Error::new(
            packet_attr.span,
            "missing `side = PacketSide::...` value from `packet` attribute",
        ));
    };

    let Expr::Path(path) = &side else {
        return Err(Error::new(packet_attr.span, "side is not enum variant"));
    };

    let name_str = if let Some(attr_name) = packet_attr.name {
        attr_name.value()
    } else {
        name_orig
    };

    let side_ident: Ident = syn::parse_str(
        &path
            .path
            .segments
            .last()
            .unwrap()
            .ident
            .to_string()
            .to_snake_case(),
    )
    .unwrap();

    let state = packet_attr
        .state
        .unwrap_or_else(|| parse_quote!(::protocol::PacketState::Play));

    let Expr::Path(path) = &state else {
        return Err(Error::new(packet_attr.span, "side is not enum variant"));
    };

    let ident: Ident = syn::parse_str(
        &(path
            .path
            .segments
            .last()
            .unwrap()
            .ident
            .to_string()
            .to_uppercase()
            + "_"
            + name_str
                .to_shouty_snake_case()
                .trim_start_matches("C_")
                .trim_start_matches("S_")),
    )
    .unwrap();

    let packet_id: Expr = match packet_attr.id {
        Some(expr) => expr,
        None => parse_quote!(::protocol::packet_id::#side_ident::#ident),
    };

    Ok(quote! {
        impl #impl_generics ::protocol::__private::Packet for #name #ty_generics
        #where_clause
        {
            const ID: (i32, ([u8; 8], u32)) = (#packet_id, ::protocol::__private::VarInt(#packet_id).encode_const());
            const NAME: &'static str = #name_str;
            const SIDE: ::protocol::PacketSide = #side;
            const STATE: ::protocol::PacketState = #state;
        }
    })
}

struct PacketAttr {
    span: Span,
    id: Option<Expr>,
    tag: Option<i32>,
    name: Option<LitStr>,
    side: Option<Expr>,
    state: Option<Expr>,
}

impl Default for PacketAttr {
    fn default() -> Self {
        Self {
            span: Span::call_site(),
            id: Default::default(),
            tag: Default::default(),
            name: Default::default(),
            side: Default::default(),
            state: Default::default(),
        }
    }
}

fn parse_packet_helper_attr(attrs: &[Attribute]) -> Result<Option<PacketAttr>> {
    for attr in attrs {
        if attr.path().is_ident("packet") {
            let mut res = PacketAttr {
                span: attr.span(),
                id: None,
                tag: None,
                name: None,
                side: None,
                state: None,
            };

            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("id") {
                    res.id = Some(meta.value()?.parse::<Expr>()?);
                    Ok(())
                } else if meta.path.is_ident("tag") {
                    res.tag = Some(meta.value()?.parse::<LitInt>()?.base10_parse::<i32>()?);
                    Ok(())
                } else if meta.path.is_ident("name") {
                    res.name = Some(meta.value()?.parse::<LitStr>()?);
                    Ok(())
                } else if meta.path.is_ident("side") {
                    res.side = Some(meta.value()?.parse::<Expr>()?);
                    Ok(())
                } else if meta.path.is_ident("state") {
                    res.state = Some(meta.value()?.parse::<Expr>()?);
                    Ok(())
                } else {
                    Err(meta.error("unrecognized packet argument"))
                }
            })?;

            return Ok(Some(res));
        }
    }

    Ok(None)
}
