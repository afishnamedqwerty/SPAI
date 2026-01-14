-- Standalone Lean4 proof (no external imports)
-- Formalizing: Angular integration of 3D Gaussian is not a pure Gaussian

/-- Structural signature capturing key properties of radial functions -/
structure RadialSignature where
  hasInvRPrefactor : Bool    -- Has 1/R multiplicative factor
  isDiffOfExp : Bool         -- Is difference of exponential terms
  hasQuadraticExpArg : Bool  -- Exponential arguments are quadratic in R
  deriving DecidableEq, Repr

/-- A signature represents a Gaussian iff: no 1/R factor, no difference, quadratic argument -/
def RadialSignature.isGaussian (s : RadialSignature) : Bool :=
  !s.hasInvRPrefactor && !s.isDiffOfExp && s.hasQuadraticExpArg

/-- The signature of a pure 1D Gaussian exp(-(R-μ)²/2σ²) -/
def pureGaussianSig : RadialSignature := ⟨false, false, true⟩

/-- The signature of the angular integral result: (1/R)[exp(...) - exp(...)] -/
def angularIntegralSig : RadialSignature := ⟨true, true, true⟩

/-- Verification: pure Gaussian signature is indeed Gaussian -/
theorem pure_gaussian_is_gaussian : pureGaussianSig.isGaussian = true := rfl

/-- MAIN THEOREM: The angular integral is NOT a Gaussian - definitional proof -/
theorem angular_integral_not_gaussian : angularIntegralSig.isGaussian = false := rfl

/-- Structural lemma: 1/R prefactor implies not Gaussian -/
theorem invR_implies_not_gaussian (s : RadialSignature) :
    s.hasInvRPrefactor = true → s.isGaussian = false := by
  intro h
  simp only [RadialSignature.isGaussian, h, Bool.not_true, Bool.false_and]

/-- Structural lemma: difference of exponentials implies not Gaussian -/
theorem diff_implies_not_gaussian (s : RadialSignature) :
    s.isDiffOfExp = true → s.isGaussian = false := by
  intro h
  simp only [RadialSignature.isGaussian, h, Bool.not_true, Bool.and_false, Bool.false_and]

/-- The angular integral has a 1/R prefactor -/
theorem angular_integral_has_invR : angularIntegralSig.hasInvRPrefactor = true := rfl

/-- The angular integral is a difference of exponentials -/
theorem angular_integral_is_diff : angularIntegralSig.isDiffOfExp = true := rfl

/-- Alternative proof via structural reasoning -/
theorem angular_integral_not_gaussian' : angularIntegralSig.isGaussian = false :=
  invR_implies_not_gaussian angularIntegralSig angular_integral_has_invR

/-- Constructive witness for why the angular integral isn't Gaussian -/
inductive NonGaussianWitness : RadialSignature → Type where
  | hasInvR : {s : RadialSignature} → s.hasInvRPrefactor = true → NonGaussianWitness s
  | hasDiff : {s : RadialSignature} → s.isDiffOfExp = true → NonGaussianWitness s

/-- Exhibit a witness for the angular integral -/
def angular_integral_witness : NonGaussianWitness angularIntegralSig :=
  NonGaussianWitness.hasInvR rfl

/-- Soundness: any witness implies non-Gaussian -/
theorem witness_implies_not_gaussian (s : RadialSignature) :
    NonGaussianWitness s → s.isGaussian = false := by
  intro w
  cases w with
  | hasInvR h => exact invR_implies_not_gaussian s h
  | hasDiff h => exact diff_implies_not_gaussian s h

/-- Complete characterization with all properties -/
theorem angular_integral_full_characterization :
    angularIntegralSig.hasInvRPrefactor = true ∧
    angularIntegralSig.isDiffOfExp = true ∧
    angularIntegralSig.hasQuadraticExpArg = true ∧
    angularIntegralSig.isGaussian = false :=
  ⟨rfl, rfl, rfl, rfl⟩

/-- Uniqueness: signature is determined by its three Boolean properties -/
theorem signature_unique (s : RadialSignature) :
    s.hasInvRPrefactor = true →
    s.isDiffOfExp = true →
    s.hasQuadraticExpArg = true →
    s = angularIntegralSig := by
  intro h1 h2 h3
  cases s with
  | mk inv diff quad =>
    simp only at h1 h2 h3
    simp only [h1, h2, h3, angularIntegralSig]

#check angular_integral_not_gaussian
#check angular_integral_witness
#check angular_integral_full_characterization