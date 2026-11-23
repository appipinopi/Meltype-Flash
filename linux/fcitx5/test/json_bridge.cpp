// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 appipinopi
// Standalone ABI test: production event adapter against the legacy parser and
// real Rust engine replies. No fcitx daemon, user profile or input is touched.
#include "../rust_json.hpp"
#include "../compat/json_reader.hpp"
#include "../compat/text.hpp"
#include <dlfcn.h>
#include <fstream>
#include <iostream>
#include <stdexcept>
#include <tuple>
using namespace meltype_fcitx;
static void check(bool condition, const std::string &message) { if (!condition) throw std::runtime_error(message); }
static bool same(const Json &a, const Json &b) {
    if (a.kind != b.kind) return false;
    switch (a.kind) {
    case Json::Kind::Null: return true;
    case Json::Kind::Bool: return a.boolean == b.boolean;
    case Json::Kind::Number: return a.number == b.number;
    case Json::Kind::String: return a.string == b.string;
    case Json::Kind::Array:
        if (a.items.size() != b.items.size()) return false;
        for (size_t i=0; i<a.items.size(); i++) if (!same(a.items[i],b.items[i])) return false;
        return true;
    case Json::Kind::Object:
        if (a.members.size() != b.members.size()) return false;
        for (const auto &member:a.members) { const auto *other=b.get(member.first); if (!other || !same(member.second,*other)) return false; }
        return true;
    }
    return false;
}
using ViewEvent=std::tuple<uint32_t,int,std::string>;
using ViewTrace=std::vector<ViewEvent>;
using ViewCallback=int (*)(void *,uint32_t,int,const uint8_t *,uint32_t);
static int collectView(void *context,uint32_t op,int arg,const uint8_t *text,uint32_t length) noexcept {
    try { static_cast<ViewTrace *>(context)->emplace_back(op,arg,std::string(reinterpret_cast<const char *>(text),length)); return 1; }
    catch(...) { return 0; }
}
// Independent panel specification, including typing and prediction lists.
static ViewTrace referenceView(const Json &result) {
    ViewTrace out;
    const auto emit=[&](uint32_t op,int arg=0,std::string text={}) {out.emplace_back(op,arg,std::move(text));};
    auto *view=result.get("view");
    if(!view || view->kind!=Json::Kind::Object) {emit(8);return out;}
    emit(0);
    const auto text=view->str("text");
    const auto clauses=view->strings("clauses");
    const bool converting=view->flag("converting");
    const int selectedClause=view->integer("selectedClause",-1);
    if(converting&&!clauses.empty()) {
        for(size_t i=0;i<clauses.size();i++)emit(1,1|(static_cast<int>(i)==selectedClause?2:0),clauses[i]);
    } else emit(1,1,text);
    emit(2,!text.empty()?1:0);
    auto hint=view->str("hint");
    const auto suggestion=view->str("suggestion");
    if(!suggestion.empty())hint=suggestion+"　"+hint;
    if(!hint.empty())emit(3,0,hint);
    auto candidates=view->strings("candidates");
    const bool predicting=!converting&&candidates.size()<=1;
    if(predicting)candidates=view->strings("predictions");
    if((predicting&&!candidates.empty())||(!predicting&&candidates.size()>1)) {
        emit(4,9);
        for(size_t i=0;i<candidates.size();i++)emit(5,static_cast<int>(i),candidates[i]);
        const int selected=view->integer(predicting?"selectedPrediction":"selectedIndex",predicting?-1:0);
        emit(6,selected>=0&&selected<static_cast<int>(candidates.size())?selected:-1);
    }
    emit(7);return out;
}
#include "edit_reference.hpp"
int main(int argc,char **argv) {
    try {
        check(argc==3,"library and private profile required");
        void *library=dlopen(argv[1],RTLD_NOW|RTLD_LOCAL);check(library!=nullptr,"Rust library load failed");
        auto visit=reinterpret_cast<JsonVisit>(dlsym(library,"meltype_json_visit_utf8"));check(visit!=nullptr,"UTF-8 visitor missing");
        auto context=reinterpret_cast<int (*)(const uint8_t *,uint32_t,uint32_t,uint32_t,uint32_t *)>(dlsym(library,"meltype_fcitx_context_offsets"));
        auto deletion=reinterpret_cast<int (*)(const uint8_t *,uint32_t,uint32_t,const uint8_t *,uint32_t,int)>(dlsym(library,"meltype_fcitx_delete_count"));
        check(context&&deletion,"text ABI missing");
        size_t textCases=0;
        for(const std::string &input:{std::string("abcdef"),std::string("先前😀後かな"),std::string("a\0b",3),std::string(70,'a')}) {
            for(size_t cursor=0;cursor<=charLength(input);cursor++) {
                for(uint32_t limit:{0U,1U,2U,20U,100U}) {
                    uint32_t offsets[3]={123,123,123};
                    check(context(reinterpret_cast<const uint8_t *>(input.data()),input.size(),cursor,limit,offsets)==1,"context rejected");
                    const auto start=cursor>limit?cursor-limit:0;
                    check(offsets[0]==byteOffset(input,start)&&offsets[1]==byteOffset(input,cursor)&&offsets[2]==byteOffset(input,cursor+20),"legacy context offset mismatch");
                    textCases++;
                }
                for(const std::string &expected:{std::string("a"),std::string("bc"),std::string("前😀"),std::string("かな"),std::string("a\0b",3)}) {
                    for(int remove:{1,2,3,4}) {
                        const auto start=cursor>charLength(expected)?cursor-charLength(expected):0;
                        const auto before=input.substr(byteOffset(input,start),byteOffset(input,cursor)-byteOffset(input,start));
                        const int count=utf16Length(expected)==static_cast<size_t>(remove)&&before.size()>=expected.size()
                            &&before.compare(before.size()-expected.size(),expected.size(),expected)==0?static_cast<int>(charLength(expected)):0;
                        check(deletion(reinterpret_cast<const uint8_t *>(input.data()),input.size(),cursor,reinterpret_cast<const uint8_t *>(expected.data()),expected.size(),remove)==count,"legacy deletion guard mismatch");
                        textCases++;
                    }
                }
            }
        }
        uint32_t unchanged[3]={123,123,123};const std::string broken("\xff",1);
        check(context(reinterpret_cast<const uint8_t *>(broken.data()),broken.size(),0,20,unchanged)==0&&unchanged[0]==123&&unchanged[1]==123&&unchanged[2]==123,"invalid UTF8 must not write offsets");
        check(context(reinterpret_cast<const uint8_t *>(""),0,0,20,unchanged)==0,"empty surrounding text rejected");
        check(context(nullptr,0,0,20,unchanged)==0,"null text rejected");
        check(deletion(reinterpret_cast<const uint8_t *>(broken.data()),broken.size(),1,reinterpret_cast<const uint8_t *>("a"),1,1)==0,"invalid surrounding UTF8 rejected");
        const std::string valid="かな";
        check(deletion(reinterpret_cast<const uint8_t *>(valid.data()),valid.size(),2,reinterpret_cast<const uint8_t *>(broken.data()),broken.size(),1)==0,"invalid expected UTF8 rejected");
        check(deletion(reinterpret_cast<const uint8_t *>(valid.data()),valid.size(),2,reinterpret_cast<const uint8_t *>(""),0,0)==0,"empty expected text rejected");
        check(context(reinterpret_cast<const uint8_t *>(valid.data()),valid.size(),99,20,unchanged)==0,"invalid cursor rejected");
        std::cout<<"fcitx Rust text: "<<textCases<<" legacy context/deletion cases, UTF-8/UTF-16/character units and invalid input passed\n";
        auto viewPlan=reinterpret_cast<int (*)(const uint8_t *,uint32_t,void *,ViewCallback)>(dlsym(library,"meltype_fcitx_view_utf8"));
        check(viewPlan!=nullptr,"view ABI missing");
        size_t viewCount=0;
        const auto compareView=[&](const std::string &text) {
            Json reference; check(JsonReader(text).read(reference),"view reference parse");
            ViewTrace actual;
            check(viewPlan(reinterpret_cast<const uint8_t *>(text.data()),text.size(),&actual,collectView)==1,"view traversal");
            check(actual==referenceView(reference),"view policy/reference mismatch"); viewCount++;
        };
        for(const std::string text:{"{}","null",R"({"view":[]})",R"({"view":{}})",R"({"view":{"text":"","clauses":["仮名"],"converting":true,"candidates":["仮名"],"hint":"案内"}})",
            R"({"view":{"text":false,"clauses":[null],"converting":"true","candidates":[],"suggestion":false}})",
            R"({"view":{"text":"かな","converting":true,"candidates":["仮名","かな"],"selectedIndex":1.9,"selectedClause":true}})",
            R"({"view":{"text":"a\u0000😀","clauses":[false,"一","二"],"converting":true,"selectedClause":1,"candidates":["甲",null,"乙"],"suggestion":"もしかして","hint":"案内"}})"})compareView(text);
        for(int selected=-3;selected<15;selected++) {
            for(const auto converting:{"true","false","null"}) {
                for(const auto clauses:{"[]",R"(["仮","名"])",R"([3,null,"かな"])","null"})
                    compareView(std::string(R"({"view":{"text":"かな","clauses":)")+clauses+",\"converting\":"+converting+",\"selectedClause\":"+std::to_string(selected)+",\"selectedIndex\":"+std::to_string(selected)+R"(,"candidates":["仮名","かな"],"suggestion":"候補","hint":"案内"}})");
            }
        }
        // Explicit new-policy cases also assert list contents and selection,
        // so the differential reference cannot silently agree on a missing list.
        for(const bool typing:{false,true}) {
            const std::string text=typing
                ? R"({"view":{"text":"かな","converting":false,"candidates":["仮名","かな"],"predictions":["予測"],"selectedIndex":1,"selectedPrediction":0}})"
                : R"({"view":{"text":"かな","converting":false,"candidates":["かな"],"predictions":["予測"],"selectedPrediction":0}})";
            compareView(text);
            ViewTrace actual;
            check(viewPlan(reinterpret_cast<const uint8_t *>(text.data()),text.size(),&actual,collectView)==1,"typing/prediction panel");
            std::vector<std::string> items;int selected=-2;
            for(const auto &[op,arg,value]:actual) {if(op==5)items.push_back(value);if(op==6)selected=arg;}
            check(items==(typing?std::vector<std::string>{"仮名","かな"}:std::vector<std::string>{"予測"}),"explicit candidate contents");
            check(selected==(typing?1:0),"explicit candidate selection");
        }
        ViewTrace invalidView;
        const std::string invalidPlan=R"({"view":{},"bad":)";
        check(viewPlan(reinterpret_cast<const uint8_t *>(invalidPlan.data()),invalidPlan.size(),&invalidView,collectView)==0&&invalidView.empty(),"invalid JSON must not start view callbacks");
        auto stopView=[](void *context,uint32_t,int,const uint8_t *,uint32_t)->int {++*static_cast<int *>(context);return 0;};
        int callbacks=0;const std::string emptyView=R"({"view":{}})";
        check(viewPlan(reinterpret_cast<const uint8_t *>(emptyView.data()),emptyView.size(),&callbacks,stopView)==0&&callbacks==1,"view callback cancellation");
        auto applyPlan=reinterpret_cast<int (*)(void *,const uint8_t *,uint32_t,void *,ViewCallback)>(dlsym(library,"meltype_fcitx_apply_utf8"));
        check(applyPlan!=nullptr,"editor application ABI missing");
        size_t editCount=0;
        const auto compareApply=[&](const std::string &reply) {
            Json parsed;check(JsonReader(reply).read(parsed),"reference edit parse");
            for(const std::string &input:{std::string("prefix前😀後"),std::string("abc"),std::string(""),std::string("a\0b",3)}) {
                for(size_t cursor=0;cursor<=charLength(input);cursor++) {
                    for(bool capable:{false,true}) {
                        Editor actual{input,cursor,capable,deletion,false,-1,{}};
                        Editor reference{input,cursor,capable,deletion,true,-1,{}};
                        referenceApply(reference,parsed);
                        check(applyPlan(nullptr,reinterpret_cast<const uint8_t *>(reply.data()),reply.size(),&actual,editEvent)==1,"apply callback traversal");
                        check(actual.trace==reference.trace&&actual.text==reference.text&&actual.cursor==reference.cursor,"legacy edit order/content mismatch");editCount++;
                    }
                }
            }
        };
        for(const std::string reply:{"null",R"({"commits":null})",R"({"commits":[null,{},false,{"text":""},{"text":3}]})",
            R"({"commits":[{"text":"日本😀"}],"view":{"text":"かな"}})",
            R"({"commits":[{"deleteBefore":3,"expect":"前😀","text":"仮名"}],"view":{"text":"残り"}})",
            R"({"commits":[{"deleteBefore":2,"expect":"前😀","text":"NG"},{"text":"later"}]})",
            R"({"commits":[{"deleteBefore":1,"expect":"","text":"NG"}]})",
            R"({"commits":[{"deleteBefore":-1,"text":"a"},{"deleteBefore":1,"expect":"a","text":"😀"},{"deleteBefore":2,"expect":"😀","text":"b"}],"view":{}})",
            R"({"commits":[{"text":"a\u0000b"},{"deleteBefore":3,"expect":"a\u0000b","text":"終"}],"view":{}})"})compareApply(reply);
        const std::string editReply=R"({"commits":[{"deleteBefore":3,"expect":"前😀","text":"仮名"}],"view":{"text":"残り"}})";
        for(int fail:{8,9,10,11,0,1,2,7}) {
            Editor editor{"前😀",2,true,deletion,false,fail,{}};
            check(applyPlan(nullptr,reinterpret_cast<const uint8_t *>(editReply.data()),editReply.size(),&editor,editEvent)==0,"host failure must stop application");
            check(!editor.trace.empty()&&static_cast<int>(std::get<0>(editor.trace.back()))==fail,"host failure continued callbacks");
        }
        Editor invalidEditor{"前😀",2,true,deletion,false,-1,{}};
        check(applyPlan(nullptr,reinterpret_cast<const uint8_t *>(invalidPlan.data()),invalidPlan.size(),&invalidEditor,editEvent)==0&&invalidEditor.trace.empty(),"invalid JSON changed editor");
        size_t count=0;
        auto compare=[&](const std::string &text) {
            compareView(text);compareApply(text);
            Json oldValue,newValue;
            check(JsonReader(text).read(oldValue),"legacy fixture parse failed: "+text);
            check(readRustJson(text,newValue,visit),"Rust fixture parse failed: "+text);
            check(same(oldValue,newValue),"legacy/Rust value mismatch: "+text);count++;
        };
        for (const std::string text:{"null","true","false","123.5","-2.25e3","[]","{}",
            R"({"consumed":true,"commits":[{"text":"日本😀","deleteBefore":3,"expect":"前😀"}],"view":{"text":"かな","clauses":["仮名","です"],"candidates":["仮名","かな"],"converting":true,"selectedIndex":1,"selectedClause":0,"hint":"案内","suggestion":"候補"}})",
            R"({"escaped":"\u304b\u306a\ud83d\ude00\u0000\n\t\"\\","items":[null,false,0,-1,{"a":[]}]})"}) compare(text);
        for (int i=0;i<200;i++) compare("{\"selectedIndex\":"+std::to_string(i)+",\"text\":\"候補😀\",\"candidates\":[\"一\",\"二\"]}");
        for (const std::string invalid:{"", "{", "true trailing", "{\"x\":1,}", "[1,]", "{\"x\":\"\\uXXXX\"}", "1e9999"}) {
            Json out;out.kind=Json::Kind::String;out.string="unchanged";
            check(!readRustJson(invalid,out,visit),"invalid JSON accepted");check(out.string=="unchanged","failed parse changed projection");
        }
        Json unicode;check(!readRustJson(R"("\ud800")",unicode,visit),"unpaired escape rejected");
        Json duplicate;check(readRustJson(R"({"x":1,"x":2})",duplicate,visit)&&duplicate.integer("x")==2,"Rust duplicate-key policy");
        auto stopped=[](void *,uint32_t,double,const uint8_t *,uint32_t)->int{return 0;};
        const std::string text="[1]";check(visit(reinterpret_cast<const uint8_t *>(text.data()),text.size(),nullptr,stopped)==0,"callback cancellation");
        auto create=reinterpret_cast<void *(*)(const char *)>(dlsym(library,"meltype_create_at"));
        auto key=reinterpret_cast<char *(*)(void *,int,int,int,const char *,const char *)>(dlsym(library,"meltype_handle_key"));
        auto commit=reinterpret_cast<char *(*)(void *)>(dlsym(library,"meltype_commit"));
        auto destroy=reinterpret_cast<void (*)(void *)>(dlsym(library,"meltype_destroy"));
        auto freeResult=reinterpret_cast<void (*)(void *)>(dlsym(library,"meltype_free"));
        check(create&&key&&commit&&destroy&&freeResult,"core ABI missing");
        void *session=create(argv[2]);check(session!=nullptr,"private session");
        for (char scalar:std::string("nihongo")) {char *reply=key(session,0,scalar,0,nullptr,nullptr);check(reply!=nullptr,"engine key");compare(reply);freeResult(reply);}
        char *reply=commit(session);check(reply!=nullptr,"engine commit");compare(reply);freeResult(reply);
        reply=key(session,0,'k',0,nullptr,nullptr);check(reply!=nullptr,"pending key for reset");freeResult(reply);
        const std::string mismatch=R"({"commits":[{"deleteBefore":1,"expect":"x","text":"must not commit"}],"view":{"text":"must not display"}})";
        Editor mismatchEditor{"a",1,true,deletion,false,-1,{}};
        check(applyPlan(session,reinterpret_cast<const uint8_t *>(mismatch.data()),mismatch.size(),&mismatchEditor,editEvent)==1,"mismatch reset traversal");
        reply=commit(session);check(reply!=nullptr,"post mismatch commit");Json resetResult;check(readRustJson(reply,resetResult,visit),"reset reply parse");freeResult(reply);
        const auto *resetCommits=resetResult.get("commits");
        check(!resetCommits||resetCommits->items.empty(),"mismatch did not clear pending core input");
        destroy(session);
        check(dlclose(library)==0,"library close");
        std::cout<<"fcitx Rust edits: "<<editCount<<" legacy/core live-editor cases, Unicode deletions, multi-edit ordering, mismatch reset and callback failures passed\n";
        std::cout<<"fcitx Rust view: "<<viewCount<<" legacy/core panel plans, Unicode/NUL, clauses, highlight, hint, candidates, invalid input and cancellation passed\n";
        std::cout<<"fcitx Rust JSON: "<<count<<" reference/core projections, UTF-8/NUL/escapes, malformed input, cancellation and duplicate policy passed\n";
    } catch (const std::exception &error) {std::cerr<<error.what()<<'\n';return 1;}
}
