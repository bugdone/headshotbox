# Setup
Install & configure hlae with reshade and ffmpeg https://github.com/advancedfx/ReShade_advancedfx/wiki

# movie.cfg example
//Custom Presets
mirv_streams settings add ffmpeg p5 "-c:v libx264 -preset ultrafast -qp 0 {QUOTE}{AFX_STREAM_PATH}\video.mp4{QUOTE}";
mirv_streams settings add ffmpeg p4 "-c:v libx264 -preset superfast -qp 6 {QUOTE}{AFX_STREAM_PATH}\video.mp4{QUOTE}";
mirv_streams settings add ffmpeg p3 "-c:v libx264 -preset superfast -qp 12 {QUOTE}{AFX_STREAM_PATH}\video.mp4{QUOTE}";
mirv_streams settings add ffmpeg p2 "-c:v libx264 -preset superfast -qp 18 {QUOTE}{AFX_STREAM_PATH}\video.mp4{QUOTE}";
mirv_streams settings add ffmpeg p1 "-c:v libx264 -preset superfast -crf 18 {QUOTE}{AFX_STREAM_PATH}\video.mp4{QUOTE}";
mirv_streams settings add ffmpeg p0 "-c:v libx264 -preset superfast -crf 23 {QUOTE}{AFX_STREAM_PATH}\video.mp4{QUOTE}";

net_graph 0
cl_draw_only_deathnotices 1
CL_DRAWHUD_FORCE_DEATHNOTICES 1
cl_clock_correction 0

sv_skyname vertigoblue_hdr

mirv_fix playerAnimState 1
mirv_streams record matPostprocessEnable 1
mirv_streams record matDynamicTonemapping 1
mirv_streams record matMotionBlurEnabled 0
mirv_streams record matForceTonemapScale 0

mirv_streams add baseFx HSBOX
mirv_streams edit HSBOX reshade enabled 1
mirv_streams edit HSBOX settings p1
mirv_streams preview HSBOX
mirv_snd_timescale 1
mirv_deathmsg cfg noticeLifeTime 200
mirv_gameoverlay enable 0

host_framerate 60
host_timescale 0

# ini referenced in ReShade.ini
Effects=Tonemap.fx,Vibrance.fx
Techniques=Tonemap,Vibrance
TechniqueSorting=LeiFx_Tech,AdaptiveFog,AdaptiveSharpen,AmbientLight,ASCII,BloomAndLensFlares,Border,Cartoon,Chromakey,CA,CinematicDOF,Clarity,ColorMatrix,Colourfulness,AdvancedCRT,Curves,Daltonize,Deband,KNearestNeighbors,NonLocalMeans,Cross_Cursor,Depth3D,DepthHaze,DisplayDepth,RingDOF,MagicDOF,GP65CJ042DOF,MatsoDOF,MartyMcFlyDOF,DPX,Emphasize,EyeAdaption,HDR,MotionBlur,FilmGrain,FilmGrain2,FilmicAnamorphSharpen,FilmicPass,Mode1,Mode2,Mode3,FXAA,GaussianBlur,GlitchB,HighPassSharp,HQ4X,HSLShift,Layer,Levels,LiftGammaGain,LightDoF_AutoFocus,LightDoF_Far,LightDoF_Near,LumaSharpen,LUT,MagicBloom,Monochrome,MultiLUT,MXAO,Nightvision,Nostalgia,PerfectPerspective,ChromaticAberration,ReflectiveBumpmapping,Tint,SMAA,Before,After,SurfaceBlur,Technicolor,Technicolor2,TiltShift,Tonemap,UIDetect,UIDetect_Before,UIDetect_After,UIMask_Top,UIMask_Bottom,Vibrance,Vignette

[Border.fx]
border_width=0.000000,0.000000
border_ratio=2.350000
border_color=0.000000,0.000000,0.000000

[CinematicDOF.fx]
ManualFocusPlane=10.000000
UseAutoFocus=1.000000
UseMouseDrivenAutoFocus=1.000000
AutoFocusTransitionSpeed=0.200000
FNumber=4.600000
AutoFocusPoint=0.500000,0.500000
FocalLength=100.000000
ShowOutOfFocusPlaneOnMouseDown=1.000000
HighlightType=1.000000
OutOfFocusPlaneColor=0.800000,0.800000,0.800000
OutOfFocusPlaneColorTransparency=0.700000
FocusPlaneColor=0.000000,0.000000,1.000000
FocusCrosshairColor=1.000000,0.000000,1.000000,1.000000
FarPlaneMaxBlur=1.000000
NearPlaneMaxBlur=1.000000
BlurQuality=5.000000
PostBlurSmoothing=0.000000
HighlightThresholdFarPlane=0.500000
HighlightEdgeBias=0.000000
HighlightGainFarPlane=0.000000
HighlightGainNearPlane=0.000000
HighlightThresholdNearPlane=0.500000
ShowCoCValues=0.000000

[Curves.fx]
Mode=0.000000
Formula=4.000000
Contrast=0.650000

[Tonemap.fx]
Gamma=0.700000
Bleach=0.000000
Defog=0.000000
Exposure=0.200000
Saturation=0.500000
FogColor=0.000000,0.000000,1.000000

[Vignette.fx]
Type=0.000000
Ratio=1.000000
Center=0.500000,0.500000
Radius=2.000000
Amount=-1.000000
Slope=2.000000

[Colourfulness.fx]
colourfulness=1.140000
lim_luma=0.100000

[ASCII.fx]
Ascii_spacing=1.000000
Ascii_font=1.000000
Ascii_font_color=1.000000,1.000000,1.000000
Ascii_font_color_mode=1.000000
Ascii_background_color=0.000000,0.000000,0.000000
Ascii_invert_brightness=0.000000
Ascii_swap_colors=0.000000
Ascii_dithering=1.000000
Ascii_dithering_intensity=2.000000
Ascii_dithering_debug_gradient=0.000000

[Bloom.fx]
bGodrayEnable=0.000000
iBloomMixmode=2.000000
fBloomSaturation=0.800000
fBloomThreshold=0.800000
fBloomAmount=0.800000
fLensdirtSaturation=2.000000
fBloomTint=0.700000,0.800000,1.000000
fLensdirtIntensity=0.400000
bLensdirtEnable=0.000000
fFlareLuminance=0.095000
iLensdirtMixmode=1.000000
fLensdirtTint=1.000000,1.000000,1.000000
bLenzEnable=0.000000
bAnamFlareEnable=0.000000
fAnamFlareCurve=1.200000
fAnamFlareThreshold=0.900000
fAnamFlareWideness=2.400000
fAnamFlareAmount=14.500000
fAnamFlareColor=0.012000,0.313000,0.588000
fLenzIntensity=1.000000
fLenzThreshold=0.800000
bChapFlareEnable=0.000000
fChapFlareTreshold=0.900000
iChapFlareCount=15.000000
fChapFlareDispersal=0.250000
fChapFlareSize=0.450000
fFlareIntensity=2.070000
fChapFlareCA=0.000000,0.010000,0.020000
fChapFlareIntensity=100.000000
fGodrayDecay=0.990000
fGodrayExposure=1.000000
fGodrayWeight=1.250000
fGodrayDensity=1.000000
iGodraySamples=128.000000
fGodrayThreshold=0.900000
fFlareBlur=200.000000
fFlareTint=0.137000,0.216000,1.000000

[Cartoon.fx]
Power=10.000000
EdgeSlope=6.000000

[Clarity.fx]
ClarityRadius=4.000000
ClarityBlendMode=2.000000
ClarityDarkIntensity=0.400000
ClarityOffset=2.000000
ClarityBlendIfDark=50.000000
ClarityBlendIfLight=205.000000
ClarityStrength=0.400000
ClarityViewMask=0.000000
ClarityViewBlendIfMask=0.000000
ClarityLightIntensity=0.000000

[ColorMatrix.fx]
ColorMatrix_Red=0.817000,0.183000,0.000000
ColorMatrix_Green=0.333000,0.667000,0.000000
ColorMatrix_Blue=0.000000,0.125000,0.875000
Strength=1.000000

[AdaptiveSharpen.fx]
D_compr_low=0.250000
L_compr_low=0.167000
curve_height=1.000000
curveslope=0.500000
D_overshoot=0.009000
L_overshoot=0.003000
D_compr_high=0.500000
scale_cs=0.056000
L_compr_high=0.334000
scale_lim=0.100000
pm_p=0.700000

[HQ4X.fx]
s=1.500000
k=-1.100000
mx=1.000000
max_w=0.750000
min_w=0.030000
lum_add=0.330000

[AmbientLight.fx]
alAdaptBaseMult=1.000000
alDebug=0.000000
alInt=10.150000
AL_DirtTex=0.000000
AL_Adaptive=1.000000
alThreshold=15.000000
AL_Adaptation=0.000000
AL_Dirt=1.000000
alAdapt=0.700000
alLensThresh=0.500000
alAdaptBaseBlackLvL=2.000000
AL_Vibrance=0.000000
alDirtInt=1.000000
alDirtOVInt=1.000000
AL_Lens=0.000000
alLensInt=2.000000

[FakeHDR.fx]
HDRPower=1.300000
radius1=0.793000
radius2=0.870000

[Chromakey.fx]
Color=0.000000
Pass=0.000000
Threshold=0.100000
CustomColor=1.000000,0.000000,0.000000

[CRT.fx]
Amount=1.000000
ScanlineGaussian=1.000000
Resolution=1.150000
Gamma=2.400000
Brightness=0.900000
Curvature=0.000000
MonitorGamma=2.200000
ScanlineIntensity=2.000000
CurvatureRadius=1.500000
CornerSize=0.010000
ViewerDistance=2.000000
Angle=0.000000,0.000000
Overscan=1.010000
Oversample=1.000000

[Daltonize.fx]
Type=0.000000

[FineSharp.fx]
sstr=2.000000
cstr=0.900000
xstr=0.190000
pstr=1.272000
xrep=0.250000
lstr=1.490000

[Deband.fx]
Threshold=0.004000
Range=16.000000
Iterations=1.000000
Grain=0.006000

[AdaptiveFog.fx]
FogColor=0.900000,0.900000,0.900000
FogCurve=1.500000
MaxFogFactor=0.800000
FogStart=0.050000
BloomPower=10.000000
BloomThreshold=10.250000
BloomWidth=0.200000

[3DFX.fx]
DITHERAMOUNT=0.500000
DITHERBIAS=-1.000000
LEIFX_PIXELWIDTH=1.500000
LEIFX_LINES=1.000000
GAMMA_LEVEL=1.000000

[ChromaticAberration.fx]
Shift=2.500000,-0.500000
Strength=0.500000

[Depth3D.fx]
Divergence=25.000000
Depth_Map=0.000000
Depth_Map_Adjust=7.500000
ZPD_GUIDE=0.000000
Offset=0.000000
ZPD=0.010000
Depth_Map_View=0.000000
Auto_Depth_Range=0.000000
Depth_Map_Flip=0.000000
Weapon_Scale=0.000000
Weapon_Adjust=0.000000,2.000000,1.500000
Stereoscopic_Mode=0.000000
Anaglyph_Desaturation=1.000000
Perspective=0.000000
Eye_Swap=0.000000
Cross_Cursor_Adjust=255.000000,255.000000,255.000000,25.000000

[DepthHaze.fx]
EffectStrength=0.900000
FogStart=0.200000
FogColor=0.800000,0.800000,0.800000
FogFactor=0.200000

[DisplayDepth.fx]
iUIInfo=0.000000
iUIDepthSetup=0.000000
bUIUpsideDown=0.000000
bUIUsePreprocessorDefs=0.000000
bUIShowNormals=1.000000

[Layer.fx]
Layer_Blend=1.000000

[DOF.fx]
DOF_AUTOFOCUS=1.000000
fADOF_ShapeCurvatureAmount=0.300000
DOF_FARBLURCURVE=2.000000
DOF_MOUSEDRIVEN_AF=0.000000
fGPDOFBiasCurve=2.000000
fRingDOFBias=0.000000
DOF_FOCUSPOINT=0.500000,0.500000
DOF_INFINITEFOCUS=1.000000
DOF_MANUALFOCUSDEPTH=0.020000
DOF_FOCUSSAMPLES=6.000000
bADOF_ShapeApertureEnable=0.000000
DOF_NEARBLURCURVE=1.600000
DOF_FOCUSRADIUS=0.050000
bGPDOFPolygonalBokeh=1.000000
fGPDOFBrightnessMultiplier=2.000000
DOF_BLURRADIUS=15.000000
iRingDOFSamples=6.000000
iRingDOFRings=4.000000
fADOF_ShapeDiffusionAmount=0.100000
fRingDOFThreshold=0.700000
fGPDOFBrightnessThreshold=0.500000
fRingDOFGain=27.000000
fRingDOFFringe=0.500000
iMagicDOFBlurQuality=8.000000
bADOF_ImageChromaEnable=0.000000
fMagicDOFColorCurve=4.000000
iGPDOFQuality=6.000000
bMatsoDOFChromaEnable=1.000000
fADOF_SmootheningAmount=1.000000
iGPDOFPolygonCount=5.000000
fADOF_ShapeChromaAmount=0.125000
fGPDOFBias=10.000000
fGPDOFChromaAmount=0.150000
fMatsoDOFChromaPow=1.400000
fMatsoDOFBokehCurve=8.000000
iMatsoDOFBokehQuality=2.000000
fMatsoDOFBokehAngle=0.000000
fADOF_ImageChromaCurve=1.000000
iADOF_ShapeQuality=17.000000
fADOF_ShapeRotation=15.000000
bADOF_ShapeDistortEnable=0.000000
bADOF_RotAnimationEnable=0.000000
fADOF_RotAnimationSpeed=2.000000
bADOF_ShapeCurvatureEnable=0.000000
iADOF_ShapeChromaMode=3.000000
iADOF_ImageChromaHues=5.000000
fADOF_ShapeApertureAmount=0.010000
fADOF_ShapeWeightAmount=1.000000
bADOF_ShapeAnamorphEnable=0.000000
fADOF_ShapeAnamorphRatio=0.200000
fADOF_ShapeDistortAmount=0.200000
fADOF_ShapeWeightCurve=4.000000
bADOF_ShapeChromaEnable=0.000000
bADOF_ShapeDiffusionEnable=0.000000
bADOF_ShapeWeightEnable=0.000000
fADOF_BokehCurve=4.000000
fADOF_ImageChromaAmount=3.000000

[FilmicAnamorphSharpen.fx]
Strength=1.000000
Coefficient=0.000000
Offset=1.000000
Clamp=1.000000
Contrast=128.000000
Preview=0.000000

[DPX.fx]
RGB_Curve=8.000000,8.000000,8.000000
Strength=0.200000
RGB_C=0.360000,0.360000,0.340000
Contrast=0.100000
Saturation=3.000000
Colorfulness=2.500000

[Emphasize.fx]
FocusDepth=0.029000
FocusRangeDepth=0.013000
Sphere_FocusVertical=0.500000
FocusEdgeDepth=0.174000
BlendColor=0.000000,0.000000,0.000000
Spherical=0.000000
Sphere_FieldOfView=75.000000
Sphere_FocusHorizontal=0.500000
BlendFactor=0.000000
EffectFactor=0.900000

[EyeAdaption.fx]
fAdp_Speed=0.100000
bAdp_BrightenEnable=1.000000
bAdp_DarkenEnable=1.000000
fAdp_DarkenCurve=0.500000
fAdp_BrightenThreshold=0.200000
fAdp_BrightenBlack=0.500000
fAdp_BrightenCurve=1.000000
fAdp_BrightenMax=0.100000
fAdp_BrightenDynamic=0.500000
fAdp_BrightenSaturation=0.000000
fAdp_DarkenThreshold=0.300000
fAdp_DarkenMax=0.400000
fAdp_DarkenDynamic=0.500000
fAdp_DarkenWhite=0.500000
fAdp_DarkenSaturation=0.000000

[FilmGrain.fx]
Intensity=1.000000
Variance=0.000000
Mean=0.100000
SignalToNoiseRatio=6.000000

[FilmGrain2.fx]
grainamount=0.050000
coloramount=0.600000
lumamount=1.000000
grainsize=1.600000

[FilmicPass.fx]
Strength=0.850000
Linearization=0.500000
Fade=0.400000
Contrast=1.000000
Bleach=0.000000
BaseGamma=1.000000
Saturation=-0.150000
RedCurve=1.000000
GreenCurve=1.000000
BlueCurve=1.000000
BaseCurve=1.500000
EffectGamma=0.650000
EffectGammaB=1.000000
EffectGammaR=1.000000
EffectGammaG=1.000000
LumCoeff=0.212656,0.715158,0.072186

[FXAA.fx]
Subpix=0.250000
EdgeThreshold=0.125000
EdgeThresholdMin=0.000000

[GaussianBlur.fx]
GaussianBlurRadius=1.000000
GaussianBlurOffset=1.000000
GaussianBlurStrength=0.300000

[Glitch.fx]
Amount=1.000000
bUseUV=0.000000

[Vibrance.fx]
Vibrance=1.000000
VibranceRGBBalance=1.000000,1.000000,1.000000

[HighPassSharpen.fx]
HighPassSharpRadius=1.000000
HighPassSharpOffset=1.000000
HighPassViewBlendIfMask=0.000000
HighPassBlendMode=1.000000
HighPassBlendIfDark=0.000000
HighPassBlendIfLight=255.000000
HighPassSharpStrength=0.400000
HighPassDarkIntensity=1.000000
HighPassLightIntensity=1.000000
HighPassViewSharpMask=0.000000

[HSLShift.fx]
HUERed=0.750000,0.250000,0.250000
HUEGreen=0.250000,0.750000,0.250000
HUEOrange=0.750000,0.500000,0.250000
HUEYellow=0.750000,0.750000,0.250000
HUECyan=0.250000,0.750000,0.750000
HUEBlue=0.250000,0.250000,0.750000
HUEPurple=0.500000,0.250000,0.750000
HUEMagenta=0.750000,0.250000,0.750000

[Levels.fx]
BlackPoint=16.000000
WhitePoint=235.000000
HighlightClipping=0.000000

[LiftGammaGain.fx]
RGB_Lift=1.000000,1.000000,1.000000
RGB_Gamma=1.000000,1.000000,1.000000
RGB_Gain=1.000000,1.000000,1.000000

[LightDoF.fx]
fLightDoF_Width=25.000000
fLightDoF_Amount=10.000000
f2LightDoF_CA=0.000000,1.000000
f2Bokeh_AutoFocusCenter=0.500000,0.500000
bLightDoF_UseCA=0.000000
bLightDoF_AutoFocus=1.000000
fLightDoF_AutoFocusSpeed=0.100000
bLightDoF_UseMouseFocus=0.000000
fLightDoF_ManualFocus=0.000000

[LumaSharpen.fx]
sharp_strength=0.650000
pattern=1.000000
sharp_clamp=0.035000
offset_bias=1.000000
show_sharpen=0.000000

[MagicBloom.fx]
f2Adapt_Clip=0.000000,1.000000
fBloom_Intensity=1.000000
fBloom_Threshold=2.000000
fDirt_Intensity=0.000000
fExposure=0.500000
fAdapt_Sensitivity=1.000000
fAdapt_Speed=0.100000
iDebug=0.000000
iAdapt_Precision=2.700000
