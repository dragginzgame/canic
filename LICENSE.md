# TOKO STUDIOS NETWORK LICENCE
## Version 1.1

### Plain-language summary — non-binding

The full terms control.

- You may inspect, modify and distribute the software and build commercial
  applications. Compliant use carries no licence fee or revenue share payable
  to Toko under this Licence.
- Run the covered production runtime on the canonical public Internet Computer.
  Off-chain front ends, ordinary integrations, monitoring and backups can
  support that deployment. Keep its covered execution and authoritative state
  on the canonical network. Genuine isolated development and testing may run elsewhere.
- Pay actual ordinary protocol charges with verifiable ICP-origin funding.
  A production Cloud Engine covered by Section 7.2 must settle the full
  applicable fee through the engine mechanism and verifiably burn native ICP
  worth at least 20% of the unsubsidised engine price. A sponsor may pay the bill.
- Publish the exact Corresponding Source of covered modifications when
  distributed or operated in production. Distribute unmodified binaries with
  access to their exact source. Independent application code may stay private.
- Toko may separately license or migrate material within the scope of its
  rights. Third-party terms and earlier licence grants remain in effect. This
  is a source-available licence with production restrictions.

---

**Licensing Organisation:** Toko Studios Limited

**Company number or registration identifier:** IBC No. 1503915

**Place and date of incorporation:** Commonwealth of The Bahamas, 9 July 2026

**Registered office:** Bayside Executive Park, Building 3, West Bay Street and Blake Road, Nassau, Bahamas

**Licence notice address:** Bayside Executive Park, Building 3, West Bay Street and Blake Road, Nassau, Bahamas

**Licence notice email:** legal@toko.ltd

**Version date:** 25 November 2026. This version applies only to material expressly
offered under it by an authorised Copyright Holder.

**Licence identifier:** TSNL-1.1

**Intended scope:** Software and associated materials expressly designated by
their applicable Copyright Holders as licensed under TSNL-1.1.

---

## 1. Purpose, application and nature

### 1.1 Purpose

Toko Studios develops software for the Internet Computer and invests in ICP.
It offers this software on terms that tie production demand for computing
capacity to verifiable destruction of native ICP. For Cloud Engines, that
commitment applies equally to public- and private-sector workloads. Section
7.2 sets the economic condition and Section 9 sets the evidence required to
show that it was met.

This Licence permits access to, development of and commercial use of Covered
Software while preserving:

a. production use on the Canonical Internet Computer with resource funding
economically aligned to native ICP destruction;

b. public access to Covered Modifications used in production or distributed;

c. the intellectual-property rights of the applicable Copyright Holders;

d. the Organisation's permanent freedom to use, develop, distribute,
commercialise, migrate and separately license the Covered Software; and

e. protection against appropriation of Covered Software for an unauthorised
blockchain, protocol fork, private execution network or substitute production
platform.

### 1.2 Application

This Licence applies only to source code, documentation, tools, libraries,
generated materials, WebAssembly modules and other copyrightable material
expressly identified as distributed under TSNL-1.1 ("Licensed Material").

It applies regardless of programming language, compilation target,
distribution format, package structure or execution environment.

It does not automatically apply to third-party dependencies or other material
merely because that material is distributed with Licensed Material.

### 1.3 Nature of Licence

This is a source-available licence containing reciprocal source and
production-network restrictions. It is not the GNU GPL, GNU AGPL, MIT Licence
or an Open Source Initiative-approved open-source licence.

Permissions not expressly granted are reserved. Statutory exceptions and other
rights that applicable law does not permit a licence to restrict remain
unaffected.

## 2. Definitions

### 2.1 Organisation

"Organisation" means Toko Studios Limited, IBC No. 1503915, incorporated in
the Commonwealth of The Bahamas on 9 July 2026, acting only to the extent it
owns or is authorised to license the relevant rights.

The Organisation does not include a shareholder, investor, creditor, parent,
subsidiary, fund, portfolio company, node provider, infrastructure provider,
governance participant, purchaser, contractor, adviser or affiliate merely
because of that status or relationship.

### 2.2 Copyright Holder

"Copyright Holder" means a person or legal entity owning or lawfully
controlling copyright in relevant Licensed Material or a Covered Modification.
Each Copyright Holder grants permissions only to the extent of rights it
lawfully controls.

### 2.3 Licensee

"You" or "Licensee" means any individual or legal entity exercising rights
under this Licence. Entities under common control are separate Licensees unless
applicable law requires otherwise. Permission held by one entity does not pass
to another merely through control, investment, contract or affiliation.

### 2.4 Covered Software

"Covered Software" means Licensed Material and copyright-protected portions of
modifications, adaptations, translations, reproductions and derivative works of
that material.

Covered Software includes source and compiled forms, WebAssembly, native
binaries, embedded runtime components, generated implementations and material
moved between packages, services, repositories, languages, modules or
processes.

Independently created software that does not reproduce or adapt protected
expression from Licensed Material is not Covered Software merely because it
performs equivalent functions or interoperates with Covered Software.

### 2.5 Covered Modification

"Covered Modification" means any alteration, replacement, extension,
translation or adaptation of copyright-protected implementation material from
Covered Software.

A Covered Modification remains covered when reorganised, renamed, translated,
generated, compiled, split across modules or services, or combined with other
software. Independently authored application code is governed by Section 2.6.

### 2.6 Independent Application

"Independent Application" means independently authored application-specific
software that uses Covered Software through documented interfaces, APIs,
macros, schemas, generated bindings or intended integration mechanisms without
constituting a Covered Modification.

An Independent Application may remain private and proprietary. Static linking,
compilation into the same WebAssembly module, use of procedural macros or
ordinary generation of bindings does not by itself remove this exception.

This exception does not convert copied or adapted Covered Software into an
Independent Application or permit Independent Application code to conceal a
Covered Modification.

Software is not an Independent Application to the extent that it:

a. replaces, overrides or relocates internal Covered Software behaviour;

b. copies or adapts covered schemas, storage engines, runtime logic, macros,
generated implementations or internal orchestration;

c. uses a hook, adapter, sidecar, service or separate canister primarily to
perform behaviour moved out of a Covered Modification; or

d. divides one Covered Modification across applications, processes, canisters,
services, modules or generated output.

Ordinary use of documented public interfaces remains within the Independent
Application exception where it does not reproduce or relocate protected
implementation material.

### 2.7 Canonical Internet Computer

"Canonical Internet Computer" means the continuously operating public Internet
Computer mainnet identified at first publication of TSNL-1.0, governed
through its Network Nervous System, preserving continuity of its public state
history and using native ICP-based economic resource funding. The release notice should
identify the mainnet reference records used to establish continuity, including
the NNS subnet and native ICP ledger principal identifiers.

Ordinary governance-approved upgrades, system-canister replacements and
technical improvements do not create a new network while that public state,
governance and economic continuity remain.

Temporary outages or maintenance do not end canonical status. Resumption of the
same network does not confer canonical status on a separate replacement.

The following are not the Canonical Internet Computer merely by reason of
technical similarity, branding, copied history or economic arrangements:

a. a fork or copy of the protocol or network;

b. a network created from exported, copied or reconstructed state;

c. a private, sovereign, enterprise or permissioned replacement network;

d. a competing or substitute WebAssembly execution network;

e. a network governed through materially separate governance;

f. a network using a separate native asset or materially separate resource
funding; or

g. a network that copies ICP balances, identities, neurons, governance
positions, node-provider positions or other economic state.

If a protocol split creates competing continuations and the criteria above do
not identify one unambiguously, no competing continuation obtains permission
merely by claiming canonical status. The Organisation may publish a good-faith
designation for licensing administration, without claiming governance authority
over the protocol. A Licensee needing certainty before that designation must
obtain written permission under Section 11.

### 2.8 Production Operation

"Production Operation" means use of Covered Software to execute, store,
coordinate, administer or materially support genuine operational workloads.

It includes commercial services, internal business systems, enterprise
systems, publicly accessible applications and operational infrastructure,
whether or not consideration is charged.

It also includes material use of Covered Software to provision, administer or
operate a production blockchain or execution network.

It excludes the isolated non-operational development, testing, research and
validation permitted by Section 4.

### 2.9 Qualifying ICP Burn

"Qualifying ICP Burn" means actual, irreversible destruction of native ICP
through the Canonical Internet Computer's recognised ICP-to-cycles conversion
mechanism for ordinary cycle funding. An Engine Fee Burn is separately defined
in Section 7.2 and does not create qualifying cycle funding by itself.

It must be independently verifiable using appropriate canonical ICP ledger,
cycles-minting and supporting protocol records.

The following do not independently constitute Qualifying ICP Burns:

a. transfers, escrows, staking or locking of ICP;

b. destruction of wrapped, synthetic or substitute assets;

c. destruction or consumption of cycles without evidence of qualifying
originating ICP conversion;

d. locally fabricated cycles or resource credits;

e. credits issued by an alternative network;

f. accounting entries, promises or simulated transactions; or

g. token, equity, revenue-sharing or infrastructure arrangements.

Previously completed qualifying conversions may count. This Licence does not
require a second ICP destruction merely because qualifying cycles are later
transferred or consumed.

### 2.10 Public production boundary

The public production permission applies only to the Canonical Internet
Computer. A private, sovereign, enterprise, permissioned or disconnected
IC-based network, fork or replica does not qualify, even if it burns ICP.
The Organisation's separate rights and written permissions under Section 11
remain unaffected.

### 2.11 Corresponding Source

"Corresponding Source" means complete human-readable source necessary to
understand, modify, compile and reproduce Covered Software or a Covered
Modification. It includes source files, build definitions, dependency
specifications, interface definitions, generated-source inputs, patches and
necessary build instructions.

It does not require credentials, private keys, user information, production
data, unrelated trade secrets or Independent Application source.

For a combined build, Corresponding Source must permit reconstruction of the
covered components without requiring disclosure of the complete Independent
Application. Replacements such as interface descriptions or minimal build
harnesses must not conceal Covered Modifications.

### 2.12 Control and Related Party

"Control" means direct or indirect power to direct management, policies,
deployment or material technical operation, whether through equity, tokens,
governance rights, contract, debt, infrastructure custody, keys or another
arrangement.

"Related Party" means an entity or person controlling, controlled by or under
common control with a Licensee, or acting as its nominee, agent or coordinated
operator in relation to Covered Software.

### 2.13 Cloud Engine

A "Cloud Engine" is a dedicated Internet Computer subnet provisioned for a
customer or operator under the canonical protocol's engine provisioning and
rental mechanism, including a renamed or successor form of that mechanism.
Exclusive tenancy or restricted access to applications on such a subnet is
consistent with Section 5 if the subnet remains part of the Canonical Internet
Computer under Section 2.7. A separate network remains subject to Section 2.10.

## 3. General grant

Subject to this Licence, each Copyright Holder grants You a worldwide,
non-exclusive, royalty-free copyright licence to:

a. access, inspect, study and reproduce Covered Software;

b. modify Covered Software and create Covered Modifications;

c. compile Covered Software into executable forms;

d. distribute Covered Software and Covered Modifications in source or
executable form;

e. develop and commercialise Independent Applications;

f. perform Production Operation on the Canonical Internet Computer in accordance with Sections 5 through 9; and

g. perform activities expressly permitted by Section 4.

No application-revenue royalty, percentage of business profits or direct
licensing fee is owed solely for compliant Production Operation.

Network, funding, source and evidence requirements are express conditions and
limitations of the grant. No permission for another network or production
environment is granted except through written permission under Section 11.

## 4. Development, testing and ancillary use

### 4.1 Non-production use

You may operate Covered Software in isolated local development, simulations,
disposable test networks, continuous integration, security laboratories,
audits, benchmarks and non-operational validation. These environments may use
fabricated cycles and synthetic infrastructure.

### 4.2 Operational boundary

An environment is not excluded merely because it is called a test, staging,
research, preview or experimental environment. Genuine internal or third-party
operational workloads constitute Production Operation.

### 4.3 Ancillary use and independent integrations

Covered Software may execute off-chain as reasonably necessary to build,
deploy, access, administer, secure, monitor, back up or restore an authorised
canonical deployment, subject to the limits below. Off-chain includes any
environment outside the Canonical Internet Computer.

Permitted uses include:

a. browsers, wallets, mobile and desktop clients, and web front ends that
   display data, collect inputs or submit requests to the canonical deployment;

b. CDNs, caches, read-only search indexes and analytics mirrors whose data is
   derived from and can be reconciled to the canonical deployment;

c. build and release tooling, orchestration that submits deployment or
   management requests, monitoring, logging, alerting and security tooling;

d. off-site backups and isolated recovery drills, and restoration to an
   authorised canonical deployment; and

e. ordinary integrations with independently operated services, which may
   maintain authoritative records of their own distinct functions, such as
   payment processor settlement records, identity provider records or
   fulfilment records.

The canonical deployment must execute the covered production logic and remain
the final authority for the operational state that logic creates, validates or
changes. An off-chain component may prepare, cache, display or propose data
and relay requests. It may not take over the execution of covered logic, make
the operative decision in its place, or control the authoritative state of
that covered workload. Posting a hash, result or periodic copy to the
canonical network does not make an off-chain runtime or database canonical.

For example, an off-chain payment processor can report whether it received
payment and a canonical canister can apply the application's entitlement
rules. An off-chain service may not apply covered entitlement rules or hold
the authoritative entitlement ledger while the canister simply publishes the
results. A backup promoted to serve genuine operational workloads is subject
to Section 5 from the start of that operation.

Independent Applications remain subject to Section 2.6. An independently
written component does not become Covered Software merely because its use
makes a particular deployment ineligible for this public grant. Ancillary
hosting charges do not independently require an ICP burn. Section 7.2 applies
when a host tool materially provisions, administers or operates a production
Cloud Engine.

## 5. Production network authorisation

### 5.1 Permitted Production Operation

Subject to Sections 6 through 9, Production Operation is permitted only on
the Canonical Internet Computer.

Section 4.3 permits related ancillary execution. Any other Production Operation
requires a written permission under Section 11.

### 5.2 No private-network permission

Separate private, sovereign, government, enterprise, permissioned and
disconnected IC-based networks are not authorised by the public grant.
Government use of a canonical Cloud Engine is subject to the same Section 7.2
conditions as any other customer's use. Technical
compatibility, equivalent resource funding or burning ICP does not change
that restriction. Section 11 preserves the Organisation's separate authority.

### 5.3 Prohibited production environments

Without written permission under Section 11, You must not use Covered Software
for Production Operation on:

a. a blockchain or distributed execution network other than the Canonical
Internet Computer;

b. a private or permissioned IC-based deployment;

c. an unrelated alternative WebAssembly execution platform;

d. a replacement network using Covered Software, regardless of ICP burn; or

e. an off-chain or hybrid substitute outside the ancillary-use exception.

The prohibition applies regardless of whether the environment is commercial,
charitable, governmental, decentralised, publicly funded, privately owned or
free of charge.

### 5.4 No permission through status or nominal similarity

Permission is not created merely because another environment:

a. executes compatible WebAssembly or system APIs;

b. reproduces canister identifiers, governance, consensus, state or balances;

c. uses compatible libraries, tooling or interfaces;

d. allocates tokens, shares, governance rights or infrastructure interests to
ICP holders, node providers, investors or contributors;

e. is operated by a foundation, government, investor, shareholder, node
provider, infrastructure provider, affiliate or ecosystem participant;

f. burns ICP, regardless of amount; or

g. has access to repositories, internal knowledge or development processes.

### 5.5 Burn does not authorise another network

A Qualifying ICP Burn contributes to permission only when allocated to an
otherwise compliant Canonical Internet Computer deployment. A nominal, unallocated, reused or insufficient burn does
not authorise an environment or cure another breach.

### 5.6 No automatic successor or ecosystem rights

No foundation, protocol developer, governing body, government, investor, node
provider, infrastructure operator, purchaser, successor corporation or network
participant acquires additional rights merely by succeeding to, acquiring,
administering, funding or replacing part of an ecosystem.

## 6. Verifiable ICP-origin funding

### 6.1 Actual protocol charges

For Production Operation on the Canonical Internet Computer,
one hundred percent (100%) of the cycles or other protocol resource credits applied to
actual protocol charges must originate from Qualifying ICP Burns. Where those
charges use cycles, the cycles must originate from qualifying canonical
ICP-to-cycles conversion. Section 9 governs evidence of that origin.

### 6.2 Transfers and pools

Conversions by a Licensee, provider or legitimate funding source may qualify.
Transfers through canonical mechanisms do not create a second burn obligation.

Pooled records must account for qualifying inflows, other inflows, transfers,
consumption and closing balances sufficiently to prevent reuse or double
counting. Allocations of qualifying funding must not exceed qualifying credits
actually received and still available. Physical identification of fungible
units is not required where independently reviewable quantitative accounting
proves the same result.

### 6.3 Mixed and non-qualifying funding

Synthetic issuance, alternative-network credits and unsupported balances do not
qualify. Mixing qualifying and other credits does not make other credits
qualifying. Qualifying funding must exist when the protocol charges are paid;
later burns do not retrospectively qualify earlier use of non-qualifying credits.

## 7. Funding scope

### 7.1 Ordinary protocol charges

Except for the engine fee in Section 7.2, the funding requirement applies to
actual protocol charges, not imputed resource
value, infrastructure cost, application revenue, business profits or turnover.
No supplemental burn, cycle retirement, equivalent-resource valuation or monthly
resource reconciliation is required for those ordinary charges.

For charges governed by Section 7.1, where the canonical protocol makes a resource free or subsidises its charge,
only the actual remaining protocol charge, if any, is subject to Section 6.
Free queries and other zero-charge operations incur no additional burn
requirement under this Licence. Charges that are actually imposed remain
subject to Section 6 even when a provider or another party pays them.

This Section does not permit Production Operation on a private network, fork
or other blockchain. The network restriction in Section 5 and the Organisation's
separate authority under Section 11 remain unchanged.

### 7.2 Cloud Engine fee and minimum native ICP burn

This Section applies to every Cloud Engine on which You run Covered Software
in Production Operation or which You materially provision, administer or
operate in production using Covered Software, wherever that software runs.
This includes creating an engine or node set, changing production capacity,
renewing its rental or controlling its operation through an intermediary.
Ordinary client calls, compiling code and passive monitoring alone do not
trigger this Section.

For each applicable billing period, the full generally applicable engine fee
must be settled through the canonical engine-fee mechanism when due. In
addition, an Engine Fee Burn attributable to that engine and period must
irreversibly destroy native ICP with a value of at least twenty percent (20%)
of the unsubsidised price of the entire engine for that period. The same
settlement may satisfy both requirements. The 20% minimum includes the fee in
the price base and excludes separately itemised taxes. Governance or fee-rate
changes do not lower the minimum under this version.

An "Engine Fee Burn" is actual irreversible destruction of native ICP through
the canonical engine-fee mechanism or its recognised successor, attributable
to settlement of the engine fee. The payer may use any payment method that
mechanism accepts, including payment through a sponsor or reseller. A
conversion to cycles, transfer to a settlement account or payment to a node
provider does not by itself establish an Engine Fee Burn. The same native ICP
destruction must not be counted twice under this Licence.

Determine the price in this order:

a. Use the dated unsubsidised public price for the same engine, including node
   class, count, location, capacity, service commitments and term. Published
   discounts earned under the same objective criteria available to every
   customer reduce the benchmark equally whether applied at invoice or paid
   later as a rebate. Discounts tied to customer identity, affiliation or
   public-sector status do not reduce the benchmark.

b. If no usable price exists, use independently verifiable arm's-length
   offers or transactions for comparable unsubsidised capacity at the relevant
   date. Document adjustments for differences in the factors in paragraph (a).
   An independent reviewer may verify confidential transactions under
   Section 9.3.

c. Include the fair value of engine-related consideration across connected
   agreements to the extent necessary to reflect the unsubsidised price,
   including mandatory support or reservations, sponsor or reseller
   compensation and non-cash consideration for supplying the engine. Count
   each element once and exclude independently priced unrelated services.
   Apart from a discount qualifying under paragraph (a), subsidies, rebates,
   side payments, credits and waivers do not reduce the benchmark. A subsidised engine class cannot serve as its own comparator.

Use the price at purchase or renewal for the billing period. Where capacity
or the service commitment materially changes during that period, apply the
relevant dated benchmark to each affected interval using the documented
billing units. Determine the value of burned ICP in the benchmark currency
using the settlement mechanism's independently verifiable rate at settlement
or, where it supplies none, a documented independent market reference fixed
before the billing period. Use the same method consistently.

The burn must occur by the fee due date, except where the ordinary mechanism
settles in a published batch cycle. In that case the amount must be committed
to settlement when due and burned by the applicable batch settlement deadline.
A batch qualifies only to the extent its records establish the actual
irreversible burn and the allocation to each engine and period without double
counting. Section 9 governs evidence. One qualifying settlement and allocated
burn covers all Covered Software on the engine for that period.

For example, suppose the published monthly price including the fee is USD 1,000
before tax. An earned USD 100 public volume rebate under paragraph (a) gives
a benchmark of USD 900 and a minimum burn worth USD 180. A USD 100 subsidy
specific to that customer would leave the benchmark at USD 1,000 and the
minimum at USD 200. In the volume-rebate example, at a qualifying settlement
rate of USD 5 per ICP, the records must show timely payment of the full fee
and at least 36 native ICP irreversibly burned and allocated to that engine
and month by the applicable deadline. A receipt showing only cycles burned
leaves the native ICP amount and its allocation unverified.

If the benchmark cannot be independently verified, the mechanism cannot meet
the 20% floor, or the transaction and burn cannot be reconciled, this Licence
does not authorise the affected operation. Ordinary metered charges, including
chargeable calls handled by a proxy for the deployment, are subject to
Sections 6 and 7.1. This Section does not impose a separate charge on
application revenue, ordinary free queries or isolated development.

## 8. Reciprocal source obligations

### 8.1 Covered Modifications

A Licensee distributing Covered Modifications or performing Production
Operation with them must make their Corresponding Source publicly available
under this same Licence.

Production Operation includes internal business operation under Section 2.8,
even where access is restricted to employees, contractors or other authorised
internal users. Providing Corresponding Source only to those users is
insufficient; the required source must be publicly available.

Publication is required no later than first distribution or commencement of
Production Operation, whichever occurs first, without licensing fee, in a
usable machine-readable form, with build instructions and no additional
inconsistent restrictions.

Private modification and use solely for the non-production activities permitted
by Section 4.1 do not trigger publication unless the Covered Modification is
distributed.

### 8.2 Availability and identification

Corresponding Source must remain available throughout Production Operation and
for at least three years after the last distribution or operation of that
version. Material changes and deployed versions must be clearly identifiable. Publish a
versioned source manifest linking each released or deployed covered build to
its exact source revision and covered build inputs. Identify the covered
executable by a digest or equivalent independently verifiable build identifier.
For combined builds, identify the deployed build and the corresponding
covered component without disclosing Independent Application source. Each
release manifest must identify material covered by this Licence and distinguish
third-party or earlier-licensed material where different terms apply.

### 8.3 Independent Applications

Independently authored application source need not be published merely because
it links, invokes, embeds unmodified Covered Software, uses generated bindings
or communicates through inter-canister calls. Embedded Covered Modifications
remain subject to this Section.

Publication does not require submitting a pull request, contributing a
modification to an Organisation repository or obtaining the Organisation's
acceptance. The Organisation has no obligation to review, merge, maintain or
distribute a published modification.

### 8.4 Unmodified executables

A distributor of unmodified Covered Software in executable form must provide
public access to the exact Corresponding Source and identify its version. A
stable exact upstream source may be referenced while available, but the
distributor remains responsible for access during the required period and must
stop further executable distribution after discovering an outage until access
is restored.

### 8.5 Network-service notice

An operator using Covered Modifications as a service must provide a reasonably
discoverable public notice linking the service or deployment identifier to its
source location, version and build identifier. An internal service can publish
that notice externally without exposing its private endpoints, credentials or
customer data.

## 9. Evidence and records

### 9.1 Verifiable funding evidence

Each Production Operation must maintain evidence reasonably sufficient to
establish the qualifying origin of the credits used for actual protocol charges.
Canonical ledger, conversion and reconciled pool records may be used, including
records of funding supplied by a provider or another party.

Possessing, retaining and providing the evidence required by this Section is a
condition of Production Operation, subject to the correction protection in
Section 14.3. A provider assertion, invoice, balance screenshot or unexplained
summary alone does not establish qualifying origin.

### 9.2 Funding records

Records must identify the relevant deployment, qualifying ICP conversion
references, resulting credit quantities, material transfers and pool allocations,
and consumption and balances sufficient to demonstrate Section 6 compliance.
Before relying on pooled funding, the operator must secure access to a
reconciled pool record or a right to independent verification under Section
9.3. Records must tie allocations to identifiable deployments and periods,
show qualifying inflows and remaining qualifying balances, and enable a
reviewer to test allocations across the pool for double counting.
Stable canonical records may be referenced rather than copied. The same credits
must not be counted as available for more than one expenditure or allocation.

For Section 7.2, retain each engine identifier and period, the applicable dated
public terms, any comparator and adjustments, and the amounts and purpose of
any separate provider compensation, credits or rebates relevant to the price.
Keep the benchmark and fee calculation, exchange-rate source and timestamp,
engine settlement references, canonical ICP burn references and any batch
allocation records. The records must reconcile the fee paid, the benchmark,
the applicable due dates and actual native ICP destroyed. A provider may supply these records under Section 9.3. Evidence of ICP
conversion alone does not establish engine-fee settlement.

For ordinary charges under Section 7.1, no resource-equivalence measurements,
imputed query charges, cycle-retirement proofs or monthly resource reports are
required. A zero-charge operation under that section does
not require evidence of a burn for a charge that the protocol did not impose.

### 9.3 Retention, privacy and corrections

Required funding records must remain available for at least three years after
the last expenditure or transfer they substantiate. Stable
pseudonymous identifiers and proportionate confidentiality protections are
permitted, but must not prevent verification. Corrections must preserve an
auditable history.

On a reasonable, documented concern about material compliance, the Organisation
may request the supporting records reasonably necessary to verify Sections 6
through 9 or an independent audit of those records. The request must be
proportionate and must permit reasonable protection of personal data, security
secrets, government information and unrelated trade secrets, including review
by an independent professional under confidentiality where appropriate.

The Licensee must respond within thirty days or another reasonable period agreed
in writing. If the review establishes a material shortfall or deliberate
misstatement, the Licensee bears its reasonable cost; otherwise the Organisation
bears the cost of an audit it required. Neither confidentiality nor use of an
auditor permits withholding the quantitative evidence necessary to establish
compliance.

## 10. Anti-circumvention and non-appropriation

### 10.1 Substance over form

You must not knowingly evade this Licence through a Related Party, contractor,
proxy, nominee, wrapper, relay, corporate restructuring, fund, portfolio
company, repackaging, renaming, translation, code generation, module split,
service boundary, artificial allocation or accounting arrangement.

Technical, operational and economic substance prevails over labels and formal
ownership.

In determining whether a deployment is authorised, consider coordinated
components that perform covered production logic or control authoritative
state for the covered workload. Apply Section 4.3 to those components,
including databases, execution engines, sidecars, relays and backups
promoted to live service.

Funding records must account for coordinated deployments and Related Parties
where they share credits or funding pools. Splitting a service among entities,
deployment identifiers, components or accounts must not disguise non-qualifying
funding, permit double counting or authorise off-network Production Operation.

### 10.2 No rights through investment or position

Holding or acquiring equity, debt, tokens, governance power, node-provider
status, infrastructure, repository access, confidential access, employment,
contract rights or another relationship does not grant ownership of Covered
Software, authority to license it, the Organisation's reserved rights or
permission for an otherwise prohibited use.

### 10.3 No exclusive appropriation

A Licensee must not represent that it owns the Organisation's or another
Copyright Holder's underlying Covered Software, or grant an exclusive interest
in Covered Software that conflicts with rights already granted under this
Licence.

Copyright in a Licensee's original modification does not create ownership of
underlying Covered Software, abstract ideas, protocol governance, tokens or
independently owned material.

### 10.4 No duty to surveil unrelated users

An ordinary distributor or general-purpose service provider is not responsible
solely because an unrelated party later acts without its knowledge, direction
or control. This Licence creates no general duty to monitor unrelated users.

## 11. Organisation freedom to operate and separate licensing

### 11.1 Permanent Organisation grant

Each Copyright Holder who expressly makes Licensed Material or a Covered
Modification available under this Licence grants the Organisation, to the
extent of rights that holder controls, a perpetual, worldwide, non-exclusive,
royalty-free, irrevocable, transferable and sublicensable licence to:

a. use, reproduce, modify, adapt and create derivative works;

b. compile, distribute, sublicense and commercially exploit the material;

c. operate it on public, private, permissioned, sovereign, dedicated or
alternative infrastructure;

d. migrate it to or authorise it on another blockchain or execution network;

e. combine it with proprietary or separately licensed material;

f. keep the Organisation's modifications private; and

g. offer it under proprietary, commercial, source-available, open-source or
other terms selected by the Organisation.

This is a separate grant to the Organisation. Subject to Section 11.5 for
third-party Cloud Engine operations, it is not conditional on the Organisation
satisfying Sections 5 through 9.

### 11.2 Covered Modification grant

By distributing a Covered Modification under this Licence or commencing
Production Operation that requires its publication under Section 8, its
Copyright Holder expressly offers that modification under this Licence and
grants the Organisation the rights in Section 11.1, to the extent the holder
controls those rights.

No grant is implied for third-party material the holder does not control.

### 11.3 Preservation against conflicting rights

A person exercising rights under this Licence must not grant, assign, pledge,
encumber or enforce rights in Covered Software or a Covered Modification in a
way that purports to revoke, narrow or prevent the Organisation's exercise of
Sections 11 or 13.

An assignment or transfer by a granting holder must preserve the Organisation's
existing rights to the maximum extent permitted by law. The granting holder
must not knowingly represent transferred rights as free of this Licence.

### 11.4 Independent migration authority

The Organisation may exercise its reserved rights at any time. It need not wait
for the Canonical Internet Computer to fail, end or become uneconomic.

### 11.5 Written permissions for others

The Organisation may issue a separate written licence authorising identified
parties, software, networks and uses, including exceptions to public-licence
conditions. An alternative-network permission must be express and signed by the
Organisation acting with sufficient authority from relevant rights holders.

No third party obtains an exception to Section 7.2 through a delegation,
sublicence or separate permission issued under this version, including under
Sections 11.1, 11.5, 11.6 or 17.6. This limitation applies equally to an engine operated on behalf of a third
party. In exercising rights obtained under this Licence to enter a separate
agreement for third-party Cloud Engine operation, the Organisation must
include a binding condition preserving Section 7.2 and must not grant or
delegate authority to waive it. A separate grant relying entirely on
independently held rights must identify its separate authority expressly. The
Organisation retains its own development and migration rights.

Informal statements, investments, token or equity arrangements, burns,
technical compatibility and commercial relationships are not authorisation.

### 11.6 No automatic benefit

The Organisation's rights do not automatically extend to investors,
shareholders, directors, employees acting independently, affiliates, customers,
funds, portfolio companies, node providers, infrastructure providers,
contractors, purchasers or recipients. An agent acting solely for the
Organisation may exercise only the authority delegated to it.

### 11.7 Survival and priority

The Organisation's grants survive termination of other permissions,
contributor withdrawal, later cessation of distribution and, to the maximum
extent permitted by law, assignment or transfer of the granting holder's
rights.

Sections 8, 10, 12.6 and 14 must not be interpreted to reimpose a condition
expressly displaced for the Organisation or a separately licensed recipient.
Third-party rights and notices remain unaffected.

## 12. Distribution, ownership and contributions

### 12.1 Ownership

Copyright remains with applicable Copyright Holders. Publication, receipt,
repository access, contribution, operation and investment do not transfer
ownership.

The Organisation must establish its authority through ownership, assignment,
employment or contractor terms, contributor agreements or express grants.
This Licence does not acquire rights a grantor does not control.

### 12.2 Notices

Redistributed Covered Software must include this complete Licence and applicable
copyright, attribution and third-party notices. Material modifications must be
identified prominently.

### 12.3 Contributions

A contribution is covered only where its contributor has authority and
expressly offers it under this Licence or a separate agreement sufficient for
the Organisation's intended use. Employment, repository membership and pull
request submission alone do not prove authority. Before the Organisation
incorporates a contribution intended for separate licensing or migration,
it must obtain a signed assignment or signed contributor agreement from the
actual rights holder, or confirm that an existing signed instrument already
covers the contribution and grants the required rights. A contributor acting
for an employer or client must establish authority from that rights holder.
The Organisation should retain the agreement and contribution records.

### 12.4 Third-party material and earlier grants

Third-party material retains its own terms. This Licence does not revoke or
restrict rights already granted for earlier releases under MIT, Apache or
another licence. Later material may be offered exclusively under this Licence.

### 12.5 No entitlement to official development

This Licence creates no right to merge, maintain or publish modifications in an
official repository. The Organisation may control its official development and
contribution process.

### 12.6 No unrestricted sublicensing

Except through Section 11 or a separate written licence, a distributor cannot
remove these conditions or grant additional production-network permissions on
behalf of relevant Copyright Holders.

## 13. Patent rights and protection of Organisation use

### 13.1 Public patent grant

Each contributor or Copyright Holder controlling applicable patent claims
("Patent Grantor") grants You a worldwide, non-exclusive, royalty-free patent
licence for claims necessarily infringed by its covered contribution alone or
in authorised combination with Covered Software.

The grant does not cover unrelated products, independent implementations or
claims the Patent Grantor cannot license. Production use remains subject to
Sections 5 through 9.

### 13.2 Permanent Organisation patent grant

Each Patent Grantor grants the Organisation a perpetual, worldwide,
non-exclusive, royalty-free, irrevocable, transferable and sublicensable
licence under the claims described in Section 13.1 for the Organisation's
products, services, modifications, combinations, migrations and commercial
activities on any infrastructure.

This grant survives termination of other permissions and, to the maximum
extent permitted by law, later assignment of the patents.

### 13.3 Defensive termination

If You initiate, direct, finance, control or maintain a patent claim
alleging that Covered Software, a covered contribution, or the Organisation's
exercise of Sections 11 or 13 infringes a patent controlled by You, patent
permissions granted to You under Section 13.1 terminate upon assertion.

This does not apply solely to a defensive counterclaim responding to proceedings
first brought against You, a validity challenge, or an unrelated product claim.
It does not affect the Organisation's Section 13.2 rights.

### 13.4 Reservation

No patent rights are granted beyond this Section. No contributor grants patents
it does not control, and no grant removes independently applicable third-party
rights.

## 14. Noncompliance, suspension and termination

### 14.1 Conditions and unauthorised networks

Network eligibility, ICP funding, reciprocal source and
Organisation grants are express conditions limiting permission.

Alternative-network Production Operation outside Section 11 is unauthorised
from commencement. Later
payment, burning, migration or cessation does not retroactively authorise an
unqualified deployment.

### 14.2 Funding suspension

Subject to Section 14.3 for eligible documentary defects, permission for affected
Production Operation is suspended when non-qualifying credits are used to pay
actual protocol charges, a Section 7.2 fee is underpaid when due, an engine
burn is insufficient or missing at the applicable deadline, or the required
funding or settlement evidence is absent. During an eligible batch settlement
interval, evidence that the full required amount was committed on time
satisfies the interim evidence requirement. Evidence of the allocated
irreversible burn is required at the batch deadline. Operation
may resume prospectively after correction if no other condition remains unmet
and permissions have not separately terminated. Later payment or burning cannot authorise operation during a shortfall.

Reconstructed records may establish that qualifying funding already existed
when charges were paid. Later burns cannot retrospectively qualify actual use
of non-qualifying credits. No suspension arises solely from the absence of a
supplemental burn, valuation or retirement that Sections 6 and 7 do not require.

### 14.3 Administrative correction

An inadvertent documentary or source-publication defect may be corrected within
thirty days after discovery or written notice if the use was otherwise
authorised apart from that eligible defect itself, required qualifying funding
existed when actual protocol charges were paid, every applicable engine-fee payment and burn was completed in full by its
respective deadline under Section 7.2, the breach was not deliberate
and the Licensee acts promptly and in good faith.

Timely complete correction prevents suspension, termination or claims based
solely on that eligible defect. It does not excuse an unqualified deployment,
actual use of non-qualifying funding, concealment or deliberate withholding.
Further executable distribution during a known source outage remains prohibited.

### 14.4 Deliberate or material violations

Deliberate evasion, deliberate concealment or destruction of required evidence,
material evidence falsification, intentional source withholding, attempted
appropriation contrary to Section 10.3, interference with the Organisation's
granted rights, or repeated material breach permits the Organisation, for rights it owns or
is authorised to enforce, or the relevant Copyright Holder, to terminate the
responsible Licensee's permissions for the affected use and material. Such
termination takes effect upon receipt of written notice identifying the
affected permissions and the grounds. Operation outside the grant under
Section 14.1 and automatic suspension under Section 14.2 take effect under
those Sections independently of notice. Section 14.3 controls eligible defects.

Reinstatement requires express written authorisation from relevant Copyright
Holders.

### 14.5 Independent recipients and remedies

Termination of one Licensee does not terminate independently compliant
recipients or the Organisation's rights. Rights holders may pursue remedies
available under applicable law; this Licence creates no automatic penalty or
remedy beyond applicable law.

## 15. Independence, tokens and non-affiliation

References to Internet Computer, ICP, cycles, governance, system canisters and
protocol economics are technical and economic conditions. They do not imply
sponsorship, endorsement, partnership or approval by a foundation, association,
governance body, node provider or infrastructure provider.

This Licence grants the Organisation no ownership of the Internet Computer,
ICP, its governance or independently developed protocol software. A Qualifying
ICP Burn transfers no token, governance or network ownership to the
Organisation.

No trademark or trade-name rights are granted.

## 16. Disclaimer and limitation of liability

THE COVERED SOFTWARE IS PROVIDED "AS IS" AND "AS AVAILABLE", WITHOUT
WARRANTIES OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE, TITLE, NON-INFRINGEMENT, SECURITY, AVAILABILITY OR
ACCURACY.

TO THE MAXIMUM EXTENT PERMITTED BY LAW, NO COPYRIGHT HOLDER OR CONTRIBUTOR IS
LIABLE FOR INDIRECT, INCIDENTAL, SPECIAL, CONSEQUENTIAL, EXEMPLARY OR PUNITIVE
DAMAGES ARISING FROM THE COVERED SOFTWARE OR ITS USE.

TO THE MAXIMUM EXTENT PERMITTED BY LAW, NO COPYRIGHT HOLDER OR CONTRIBUTOR IS
LIABLE FOR ANOTHER CLAIM, LOSS OR DAMAGE ARISING FROM THE COVERED SOFTWARE OR
ITS USE, WHETHER IN CONTRACT, TORT OR OTHERWISE.

NOTHING EXCLUDES OR LIMITS LIABILITY THAT APPLICABLE LAW DOES NOT PERMIT TO BE
EXCLUDED OR LIMITED.

## 17. Interpretation, governing law and notices

### 17.1 Limited grant and essential conditions

This Licence grants only rights expressly stated and legally authorised.
Network eligibility, economic alignment, reciprocal source and preservation of
the Organisation's rights are essential conditions. Invalidity of one condition
must not be interpreted as affirmative permission for an otherwise unauthorised
use except where applicable law requires it.

### 17.2 Severability and no waiver

If a provision is invalid or unenforceable, remaining provisions continue to
the extent permitted by law. Delay or failure to enforce is not permanent
waiver.

### 17.3 Versions

Each numbered version is a separate legal instrument. A later version does not
alter an earlier grant unless relevant Copyright Holders expressly offer that
later version for the applicable material.

### 17.4 Governing law and forum

Subject to mandatory applicable law, this Licence is governed by the laws of the Principality of Monaco, excluding
conflict-of-laws rules to the extent permitted.

The competent courts of the Principality of Monaco have exclusive jurisdiction
over disputes arising out of or relating to this Licence where the forum
agreement is valid and enforceable, subject to mandatory applicable law.
Where it is ineffective, jurisdiction is determined under applicable law.

For managed deployments and separate written licences, the Organisation
must record affirmative acceptance by an authorised representative of the
applicable entity before enabling the deployment or granting permission. The
record must identify the entity, the representative, the date and the exact
licence text and version accepted, and capture acceptance of the forum clause.
This procedure does not impose a registration requirement on other users of
the public grant.

### 17.5 Notices

Formal notices to the Organisation under this Licence must be sent to Bayside
Executive Park, Building 3, West Bay Street and Blake Road, Nassau, Bahamas,
and legal@toko.ltd, or to replacement contacts published by the Organisation
in a durable public record. A notice of termination to a Licensee must be
delivered to its designated legal contact or otherwise by a means that
provides evidence of receipt.

### 17.6 Entire public permission

This Licence states the public permissions for Covered Software. A separate
written agreement issued by parties with sufficient authority may grant
additional permissions to its identified recipient subject to Section 11.5 for
permissions derived from this version.

---

**End of Toko Studios Network Licence — Version 1.1**

Copyright © the respective Copyright Holders identified in the applicable
Licensed Material. All rights not expressly granted are reserved.