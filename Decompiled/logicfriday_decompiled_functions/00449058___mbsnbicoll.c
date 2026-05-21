/* 00449058 __mbsnbicoll */

/* Library Function - Single Match
    __mbsnbicoll
   
   Library: Visual Studio 2003 Release */

int __cdecl __mbsnbicoll(uchar *_Str1,uchar *_Str2,size_t _MaxCount)

{
  _ptiddata p_Var1;
  pthreadmbcinfo ptVar2;
  int iVar3;
  
  p_Var1 = __getptd();
  ptVar2 = p_Var1->_tpxcptinfoptrs;
  if (ptVar2 != DAT_0046c9f8) {
    ptVar2 = ___updatetmbcinfo();
  }
  if (_MaxCount == 0) {
    return 0;
  }
  iVar3 = FUN_00449453(*(LCID *)ptVar2->mbulinfo,1,_Str1,(char *)_MaxCount,_Str2,(char *)_MaxCount,
                       ptVar2->mbcodepage);
  if (iVar3 == 0) {
    return 0x7fffffff;
  }
  return iVar3 + -2;
}
