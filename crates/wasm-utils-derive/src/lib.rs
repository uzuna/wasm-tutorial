//! wasm-utilで定形生成できるボイラープレートを生成するマクロを定義する

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
    DeriveInput, Ident, Token,
};

/// InputのSelectOptionを実装するためのマクロ
#[proc_macro_derive(Select, attributes(select))]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // DeriveInputを取得
    let input = parse_macro_input!(input as DeriveInput);

    // Enumの各Variantを取得
    let variants = get_variants(&input);

    // バリアントを元に各メソッドを生成
    let iter = gen_iter_method(&variants);
    let value = gen_value_method(&variants);
    let from_str = gen_from_str_method(&variants);
    let text = gen_text_method(&variants);

    // impl SelectOption の実装を生成
    let struct_name = input.ident;
    let expanded = quote! {

        impl SelectOption for #struct_name {
            #iter
            #value
            #from_str
            #text
        }
    };

    proc_macro::TokenStream::from(expanded)
}

// 生成に使う情報を保持する構造体
struct Variant {
    // enumの名前
    name: syn::Ident,
    // Variantの名前
    variant: syn::Variant,
    // value及びfrom_strの値
    value: String,
    // 表示用の値
    display: String,
    // Atrributes
    attrs: Vec<SelectAttr>,
}

impl Variant {
    fn variant(&self) -> &syn::Ident {
        &self.variant.ident
    }

    fn value(&self) -> &str {
        if let Some(attr) = self.attrs.first() {
            match attr {
                SelectAttr::Value(ref value) => return value,
            }
        }
        &self.value
    }
}

// アトリビュートによって上書きする情報
#[derive(Debug)]
enum SelectAttr {
    // valueの変更
    Value(String),
}

impl Parse for SelectAttr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        let name_str = name.to_string();

        // メモ: nameによって値の有無などが変わる場合、この分岐を増やすことになる
        let value = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let value: syn::LitStr = input.parse()?;
            value.value()
        } else {
            // 現時点ではKey=Valueの形式のみ使用しているのでそれ以外はエラーとする
            return Err(syn::Error::new_spanned(name, "expected `=` after `name`"));
        };
        match name_str.as_str() {
            "value" => Ok(SelectAttr::Value(value)),
            _ => Err(syn::Error::new_spanned(name, "unknown attribute")),
        }
    }
}

/// enumの中身を取り出す
fn get_variants(input: &DeriveInput) -> Vec<Variant> {
    let mut variants = Vec::new();

    if let syn::Data::Enum(ref e) = input.data {
        for variant in &e.variants {
            // select属性があるか調べて取り出す
            let mut attrs = Vec::new();
            for attr in &variant.attrs {
                // select属性があるなら中身を取り出す
                if attr.path().is_ident("select") {
                    if let Ok(attr) =
                        attr.parse_args_with(Punctuated::<SelectAttr, Token![,]>::parse_terminated)
                    {
                        for attr in attr {
                            println!("find attr {:?}", attr);
                            attrs.push(attr);
                        }
                    }
                } else {
                    continue;
                }
            }

            // Variantの情報を構成
            let name = input.ident.clone();
            variants.push(Variant {
                name,
                variant: variant.clone(),
                // valueはデフォルトでVariantの名前を小文字にしたものを使う
                value: variant.ident.to_string().to_lowercase(),
                display: variant.ident.to_string(),
                attrs,
            });
        }
    }

    variants
}

// iterメソッドの生成
fn gen_iter_method(variants: &[Variant]) -> TokenStream {
    let name = variants[0].name.clone();
    let arms = variants.iter().map(|v| {
        let variant = &v.variant();
        quote! {
            #name::#variant
        }
    });
    quote! {
        fn iter() -> &'static [#name] {
            &[#(#arms),*]
        }
    }
}

// valueメソッドの生成
fn gen_value_method(variants: &[Variant]) -> TokenStream {
    let name = variants[0].name.clone();
    let arms = variants.iter().map(|v| {
        let variant = &v.variant();
        let value = v.value();
        quote! {
            #name::#variant => #value,
        }
    });
    quote! {
        fn value(&self) -> &str {
            match self {
                #(#arms)*
            }
        }
    }
}

// from_strメソッドの生成
fn gen_from_str_method(variants: &[Variant]) -> TokenStream {
    let name = variants[0].name.clone();
    let arms = variants.iter().map(|v| {
        let value = v.value();
        let variant = &v.variant();
        quote! {
            #value => #name::#variant,
        }
    });
    quote! {
        fn from_str(value: &str) -> Self {
            match value {
                #(#arms)*
                _ => panic!("Invalid value: {}", value),
            }
        }
    }
}

// textメソッドの生成
fn gen_text_method(variants: &[Variant]) -> TokenStream {
    let name = variants[0].name.clone();
    let arms = variants.iter().map(|v| {
        let variant = &v.variant();
        let display = &v.display;
        quote! {
            #name::#variant => #display,
        }
    });
    quote! {
        fn text(&self) -> &str {
            match self {
                #(#arms)*
            }
        }
    }
}
